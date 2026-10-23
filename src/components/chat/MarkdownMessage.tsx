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
}

const CodeBlock: React.FC<CodeBlockProps> = ({ language, code }) => {
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
    <div className="aven-codeblock-wrapper my-3 rounded-md border border-[#34353C] bg-[#1E1F24] overflow-hidden text-xs">
      <div className="flex items-center justify-between px-3 py-1.5 bg-[#25262C] border-b border-[#34353C] text-[#A5A6B0]">
        <span className="font-mono text-[11px] uppercase tracking-wider">{displayLanguage}</span>
        <button
          onClick={handleCopy}
          className="flex items-center gap-1.5 px-2 py-0.5 rounded text-[11px] text-[#A5A6B0] hover:text-[#F4F4F5] hover:bg-[#34353C] transition-colors"
          title="Copy code"
          type="button"
        >
          {copied ? (
            <>
              <Check className="w-3.5 h-3.5 text-emerald-400" />
              <span className="text-emerald-400">Copied</span>
            </>
          ) : (
            <>
              <Copy className="w-3.5 h-3.5" />
              <span>Copy</span>
            </>
          )}
        </button>
      </div>
      <div className="overflow-x-auto p-3 text-[#F4F4F5] font-mono leading-relaxed select-text">
        <pre className="m-0 p-0 font-mono text-[12px] whitespace-pre">
          <code>{code}</code>
        </pre>
      </div>
    </div>
  );
};

export const MarkdownMessage: React.FC<MarkdownMessageProps> = ({ content, isStreaming }) => {
  const handleOpenLink = async (e: React.MouseEvent<HTMLAnchorElement>, href?: string) => {
    e.preventDefault();
    if (!href) return;

    // Safety check: only allow http and https protocols
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

  return (
    <div className="aven-markdown-content text-[13.5px] leading-relaxed text-[#E2E2E8]">
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        components={{
          code({ className, children, ...props }) {
            const match = /language-(\w+)/.exec(className || '');
            const codeString = String(children).replace(/\n$/, '');
            const isMultiline = codeString.includes('\n') || Boolean(match);

            if (isMultiline) {
              return <CodeBlock language={match ? match[1] : undefined} code={codeString} />;
            }

            return (
              <code
                className="px-1.5 py-0.5 rounded bg-[#202127] border border-[#34353C] text-[#E2E2E8] font-mono text-[12px]"
                {...props}
              >
                {children}
              </code>
            );
          },
          a({ href, children, ...props }) {
            return (
              <a
                href={href}
                onClick={(e) => handleOpenLink(e, href)}
                className="inline-flex items-center gap-0.5 text-[#7C7CF8] hover:underline underline-offset-2 font-medium cursor-pointer"
                title={href}
                rel="noopener noreferrer"
                {...props}
              >
                <span>{children}</span>
                <ExternalLink className="w-3 h-3 inline-block opacity-70" />
              </a>
            );
          },
          table({ children }) {
            return (
              <div className="aven-table-wrapper my-3 overflow-x-auto rounded border border-[#34353C]">
                <table className="w-full text-left border-collapse text-xs text-[#E2E2E8]">
                  {children}
                </table>
              </div>
            );
          },
          thead({ children }) {
            return <thead className="bg-[#25262C] border-b border-[#34353C]">{children}</thead>;
          },
          th({ children }) {
            return (
              <th className="px-3 py-2 font-semibold text-[#F4F4F5] border-r border-[#34353C] last:border-r-0">
                {children}
              </th>
            );
          },
          td({ children }) {
            return (
              <td className="px-3 py-2 border-t border-r border-[#34353C] last:border-r-0">
                {children}
              </td>
            );
          },
          blockquote({ children }) {
            return (
              <blockquote className="my-2.5 pl-3.5 border-l-2 border-[#7C7CF8] text-[#A5A6B0] italic">
                {children}
              </blockquote>
            );
          },
          h1({ children }) {
            return (
              <h1 className="text-lg font-semibold text-[#F4F4F5] mt-4 mb-2 pb-1 border-b border-[#34353C]/60">
                {children}
              </h1>
            );
          },
          h2({ children }) {
            return (
              <h2 className="text-base font-semibold text-[#F4F4F5] mt-3 mb-1.5 pb-0.5 border-b border-[#34353C]/40">
                {children}
              </h2>
            );
          },
          h3({ children }) {
            return <h3 className="text-sm font-semibold text-[#F4F4F5] mt-2.5 mb-1">{children}</h3>;
          },
          h4({ children }) {
            return <h4 className="text-xs font-semibold text-[#F4F4F5] mt-2 mb-1">{children}</h4>;
          },
          ul({ children }) {
            return <ul className="list-disc pl-5 my-2 space-y-1">{children}</ul>;
          },
          ol({ children }) {
            return <ol className="list-decimal pl-5 my-2 space-y-1">{children}</ol>;
          },
          li({ children }) {
            return <li className="pl-0.5">{children}</li>;
          },
          p({ children }) {
            return <p className="my-1.5 last:mb-0 first:mt-0">{children}</p>;
          },
          hr() {
            return <hr className="my-3 border-[#34353C]" />;
          },
        }}
      >
        {content}
      </ReactMarkdown>
      {isStreaming && (
        <span
          className="inline-block w-1.5 h-4 ml-1 bg-[#7C7CF8] align-middle animate-pulse rounded-sm"
          aria-label="Generating response"
        />
      )}
    </div>
  );
};
