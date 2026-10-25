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

import { preprocessStreamingMarkdown, isSafeUrl } from './markdownUtils';
export { preprocessStreamingMarkdown, isSafeUrl };

/**
 * Remark plugin to transform single line breaks inside paragraphs into `<br />` breaks.
 * Preserves intentional linebreaks in AI responses without requiring double-spacing.
 */
const remarkLineBreaks = () => (tree: any) => {
  function process(node: any) {
    if (!node || !node.children) return;
    if (node.type === 'code' || node.type === 'inlineCode' || node.type === 'table') return;

    const newChildren: any[] = [];
    for (const child of node.children) {
      if (child.type === 'text' && child.value && child.value.includes('\n') && node.type !== 'tableCell') {
        const parts = child.value.split('\n');
        for (let i = 0; i < parts.length; i++) {
          if (parts[i]) {
            newChildren.push({ type: 'text', value: parts[i] });
          }
          if (i < parts.length - 1) {
            newChildren.push({ type: 'break' });
          }
        }
      } else {
        process(child);
        newChildren.push(child);
      }
    }
    node.children = newChildren;
  }
  process(tree);
};

/**
 * Remark plugin to safely place the typing cursor strictly inside inline containers
 * (paragraphs, headings, blockquotes, or table cells).
 * Prevents invalid HTML nesting (such as `span` directly inside `table` or `ul`) which
 * caused React 19 hydration/render exceptions.
 */
const remarkSafeStreamingCursor = () => (tree: any) => {
  let placed = false;

  function appendToInline(node: any): boolean {
    if (!node || !node.children || node.children.length === 0) return false;

    for (let i = node.children.length - 1; i >= 0; i--) {
      const child = node.children[i];

      // Code blocks handle their own cursor inside pre element
      if (child.type === 'code') {
        return true;
      }

      // Safe inline containers where a <span> can validly exist
      if (['paragraph', 'heading', 'tableCell', 'blockquote'].includes(child.type)) {
        child.children.push({
          type: 'cursorMarker',
          data: {
            hName: 'span',
            hProperties: {
              className: 'aven-typing-cursor',
              'aria-hidden': 'true',
            },
          },
        });
        return true;
      }

      // Traverse nested structures (lists, table rows, emphasis, strong)
      if (['listItem', 'list', 'table', 'tableRow', 'strong', 'emphasis', 'delete'].includes(child.type)) {
        if (appendToInline(child)) {
          return true;
        }
      }
    }
    return false;
  }

  placed = appendToInline(tree);
  tree._hasCursor = placed;
};

export const MarkdownMessage: React.FC<MarkdownMessageProps> = ({ content, isStreaming }) => {
  const safeContent = preprocessStreamingMarkdown(content, isStreaming);

  const handleOpenLink = async (e: React.MouseEvent<HTMLAnchorElement>, href?: string) => {
    e.preventDefault();
    if (!href) return;

    // Safety check: only allow http:, https:, and mailto: protocols
    if (!isSafeUrl(href)) {
      console.warn('Rejected unsafe link protocol for href:', href);
      return;
    }

    try {
      if (isTauriDesktop()) {
        await openUrl(href);
      } else {
        window.open(href, '_blank', 'noopener,noreferrer');
      }
    } catch (err) {
      console.error('Invalid URL or opener failure:', err);
    }
  };

  const plugins = isStreaming
    ? [remarkGfm, remarkLineBreaks, remarkSafeStreamingCursor]
    : [remarkGfm, remarkLineBreaks];

  if (!safeContent) {
    if (isStreaming) {
      return (
        <div className="aven-markdown-content" style={{ position: 'relative' }}>
          <span className="aven-typing-cursor" aria-hidden="true" />
        </div>
      );
    }
    return null;
  }

  return (
    <div className="aven-markdown-content" style={{ position: 'relative' }}>
      <ReactMarkdown
        remarkPlugins={plugins}
        components={{
          // Eliminate invalid <pre><div>...</div></pre> wrapping by rendering children directly.
          pre({ children }) {
            return <>{children}</>;
          },
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
              <code className="aven-inline-code" {...props}>
                {children}
              </code>
            );
          },
          a({ href, children, ...props }) {
            const isSafe = href && (href.startsWith('http://') || href.startsWith('https://') || href.startsWith('mailto:'));
            const linkText = children && String(children).trim().length > 0 ? children : href || 'Link';
            return (
              <a
                href={isSafe ? href : undefined}
                onClick={(e) => handleOpenLink(e, href)}
                className="inline-flex items-center gap-0.5"
                title={href}
                rel="noopener noreferrer"
                target="_blank"
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
          li({ children, ...props }) {
            return <li {...props}>{children}</li>;
          },
          p({ children }) {
            return <p>{children}</p>;
          },
          hr() {
            return <hr style={{ margin: '14px 0', border: 'none', borderTop: '1px solid var(--border-subtle)' }} />;
          },
          input({ type, checked, ...props }) {
            if (type === 'checkbox') {
              return (
                <input
                  type="checkbox"
                  checked={checked}
                  readOnly
                  style={{
                    marginRight: '6px',
                    verticalAlign: 'middle',
                    accentColor: 'var(--accent-primary)',
                    cursor: 'default',
                  }}
                  {...props}
                />
              );
            }
            return <input type={type} checked={checked} {...props} />;
          },
        }}
      >
        {safeContent}
      </ReactMarkdown>
    </div>
  );
};
