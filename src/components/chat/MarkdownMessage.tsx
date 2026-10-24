import React, { useState } from 'react';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import { Copy, Check, ExternalLink } from 'lucide-react';
import { openUrl } from '@tauri-apps/plugin-opener';
import { isTauriDesktop } from '../../services/tauriIpc';

interface MarkdownMessageProps {
  content: string;
  isStreaming?: boolean;
}

interface CodeBlockProps {
  language?: string;
  code: string;
  isStreaming?: boolean;
}

const CodeBlock: React.FC<CodeBlockProps> = ({ language, code, isStreaming }) => {
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(code);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (err) {
      console.error('Failed to copy code:', err);
    }
  };

  const displayLanguage = language ? language.replace(/^language-/, '') : 'text';

  return (
    <div className="aven-codeblock-wrapper">
      <div className="aven-codeblock-header">
        <span style={{ fontFamily: 'var(--font-mono)', fontSize: '11px', textTransform: 'uppercase', letterSpacing: '0.05em' }}>
          {displayLanguage}
        </span>
        <button
          onClick={handleCopy}
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: '5px',
            padding: '2px 6px',
            borderRadius: 'var(--radius-sm)',
            fontSize: '11px',
            color: copied ? 'var(--status-success)' : 'var(--text-secondary)',
            background: 'transparent',
            border: 'none',
            cursor: 'pointer',
          }}
          title="Copy code"
          type="button"
        >
          {copied ? (
            <>
              <Check size={12} color="var(--status-success)" />
              <span style={{ color: 'var(--status-success)' }}>Copied</span>
            </>
          ) : (
            <>
              <Copy size={12} />
              <span>Copy</span>
            </>
          )}
        </button>
      </div>
      <div className="aven-codeblock-content">
        <pre>
          <code>{code}</code>
          {isStreaming && <span className="aven-typing-cursor" aria-hidden="true" />}
        </pre>
      </div>
    </div>
  );
};

/**
 * Preprocesses streaming markdown text to avoid exposing incomplete syntax
 * like dangling `[`, `[Label](`, or unclosed `[Label](url...` without `)`.
 */
export function preprocessStreamingMarkdown(raw: string, isStreaming?: boolean): string {
  if (!raw) return '';
  if (!isStreaming) return raw;

  let text = raw;

  // 1. If text ends with an unclosed link like `[Label](https://domain.com/path` (missing `)`)
  // Temporarily close with `)` so ReactMarkdown renders it as a valid link instead of raw syntax
  const unclosedLinkMatch = /\[([^\]]+)\]\((https?:\/\/[^\s\)]*)$/.exec(text);
  if (unclosedLinkMatch) {
    text = `${text})`;
  } else {
    // 2. If text ends with `[Label](` without any URL yet
    const unstartedUrlMatch = /\[([^\]]+)\]\($/.exec(text);
    if (unstartedUrlMatch) {
      // Just render the label text safely without raw bracket delimiters
      text = text.slice(0, unstartedUrlMatch.index) + unstartedUrlMatch[1];
    } else {
      // 3. If text ends with dangling `[`
      if (text.endsWith('[')) {
        text = text.slice(0, -1);
      }
    }
  }

  return text;
}

export const MarkdownMessage: React.FC<MarkdownMessageProps> = ({ content, isStreaming }) => {
  const safeContent = preprocessStreamingMarkdown(content, isStreaming);

  const handleOpenLink = async (e: React.MouseEvent<HTMLAnchorElement>, href?: string) => {
    e.preventDefault();
    if (!href) return;

    // Safety check: only allow http: and https: protocols
    try {
      const url = new URL(href);
      if (url.protocol !== 'http:' && url.protocol !== 'https:') {
        console.warn('Rejected unsafe link protocol:', url.protocol);
        return;
      }

      if (isTauriDesktop()) {
        await openUrl(href);
      } else {
        window.open(href, '_blank', 'noopener,noreferrer');
      }
    } catch (err) {
      console.error('Invalid URL or opener failure:', err);
    }
  };

  const plugins = [remarkGfm];

  return (
    <div className="aven-markdown-content" style={{ position: 'relative' }}>
      <ReactMarkdown
        remarkPlugins={plugins}
        components={{
          code({ className, children, ...props }) {
            const match = /language-(\w+)/.exec(className || '');
            const codeString = String(children).replace(/\n$/, '');
            const isMultiline = codeString.includes('\n') || Boolean(match);

            if (isMultiline) {
              return (
                <CodeBlock
                  language={match ? match[1] : undefined}
                  code={codeString}
                  isStreaming={isStreaming}
                />
              );
            }

            return (
              <code {...props}>
                {children}
              </code>
            );
          },
          a({ href, children, ...props }) {
            // Guarantee we never show empty link syntax `[]`
            const linkText = children && String(children).trim().length > 0 ? children : href || 'Link';
            return (
              <a
                href={href}
                onClick={(e) => handleOpenLink(e, href)}
                className="inline-flex items-center gap-0.5"
                title={href}
                rel="noopener noreferrer"
                {...props}
              >
                <span>{linkText}</span>
                <ExternalLink size={12} style={{ display: 'inline-block', opacity: 0.7, flexShrink: 0, marginLeft: '2px' }} />
              </a>
            );
          },
          table({ children }) {
            return (
              <div className="aven-table-wrapper">
                <table>
                  {children}
                </table>
              </div>
            );
          },
          thead({ children }) {
            return <thead>{children}</thead>;
          },
          tbody({ children }) {
            return <tbody>{children}</tbody>;
          },
          tr({ children }) {
            return <tr>{children}</tr>;
          },
          th({ children }) {
            return <th>{children}</th>;
          },
          td({ children }) {
            return <td>{children}</td>;
          },
          blockquote({ children }) {
            return <blockquote>{children}</blockquote>;
          },
          h1({ children }) {
            return <h1>{children}</h1>;
          },
          h2({ children }) {
            return <h2>{children}</h2>;
          },
          h3({ children }) {
            return <h3>{children}</h3>;
          },
          h4({ children }) {
            return <h4>{children}</h4>;
          },
          ul({ children }) {
            return <ul>{children}</ul>;
          },
          ol({ children }) {
            return <ol>{children}</ol>;
          },
          li({ children }) {
            return <li>{children}</li>;
          },
          p({ children }) {
            return <p>{children}</p>;
          },
          hr() {
            return <hr style={{ margin: '14px 0', border: 'none', borderTop: '1px solid var(--border-subtle)' }} />;
          },
        }}
      >
        {safeContent}
      </ReactMarkdown>
      {isStreaming && (
        <span
          className="aven-typing-cursor"
          style={{
            display: 'inline-block',
            verticalAlign: 'middle',
            marginLeft: '2px',
          }}
          aria-hidden="true"
        />
      )}
    </div>
  );
};
