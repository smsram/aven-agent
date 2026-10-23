import React, { useState, useRef, useEffect } from 'react';
import {
  ArrowUp,
  Sparkles,
  ChevronDown,
  Check,
  Plus,
  Copy,
  CheckCheck,
  Square,
  RotateCcw,
  AlertCircle,
} from 'lucide-react';
import { AgentTask } from '../../types';
import { Button } from '../../components/common/Button';
import { MarkdownMessage } from '../../components/chat/MarkdownMessage';
import { taskStore } from '../../services/taskStore';

export interface ConversationViewProps {
  task: AgentTask;
  onSendMessage: (text: string, model: string) => void;
  onNewChat: () => void;
  availableModels: string[];
}

export const ConversationView: React.FC<ConversationViewProps> = ({
  task,
  onSendMessage,
  onNewChat,
  availableModels,
}) => {
  const [replyText, setReplyText] = useState('');
  const [selectedModel, setSelectedModel] = useState(task.model || '@cf/qwen/qwen3.8-27b');
  const [showModelDropdown, setShowModelDropdown] = useState(false);
  const [copiedIndex, setCopiedIndex] = useState<number | null>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const scrollContainerRef = useRef<HTMLDivElement>(null);
  const isUserScrolledUp = useRef(false);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const isGenerating = task.status === 'running' || task.messages?.some((m) => m.status === 'thinking');

  // Track if user manually scrolls up to prevent disrupting reading
  const handleScroll = () => {
    if (!scrollContainerRef.current) return;
    const { scrollTop, scrollHeight, clientHeight } = scrollContainerRef.current;
    isUserScrolledUp.current = scrollHeight - scrollTop - clientHeight > 80;
  };

  // Auto-scroll to bottom only if user hasn't scrolled up
  useEffect(() => {
    if (!isUserScrolledUp.current) {
      messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
    }
  }, [task.messages, task.logs]);

  // Focus textarea on mount
  useEffect(() => {
    textareaRef.current?.focus();
  }, [task.id]);

  const handleSend = (e?: React.FormEvent) => {
    if (e) e.preventDefault();
    if (!replyText.trim() || isGenerating) return;
    isUserScrolledUp.current = false;
    onSendMessage(replyText.trim(), selectedModel);
    setReplyText('');
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  const handleCopyText = (text: string, index: number) => {
    navigator.clipboard.writeText(text);
    setCopiedIndex(index);
    setTimeout(() => setCopiedIndex(null), 2000);
  };

  const messages = task.messages || [
    {
      id: 'm1',
      sender: 'user' as const,
      text: task.prompt,
      timestamp: task.createdAt,
    },
    {
      id: 'm2',
      sender: 'assistant' as const,
      text: task.output || '',
      status: task.status === 'running' ? ('thinking' as const) : ('done' as const),
      timestamp: task.completedAt || task.createdAt,
      logs: task.logs,
    },
  ];

  return (
    <div
      style={{
        height: '100%',
        display: 'flex',
        flexDirection: 'column',
        overflow: 'hidden',
        backgroundColor: 'var(--bg-main)',
      }}
    >
      {/* Conversation Top Header Bar */}
      <div
        style={{
          height: '42px',
          padding: '0 20px',
          borderBottom: '1px solid var(--border-subtle)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          flexShrink: 0,
          backgroundColor: 'var(--bg-main)',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: '10px', minWidth: 0 }}>
          <button
            onClick={onNewChat}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: '4px',
              padding: '3px 8px',
              borderRadius: 'var(--radius-sm)',
              backgroundColor: 'var(--bg-panel)',
              border: '1px solid var(--border-subtle)',
              fontSize: '11px',
              color: 'var(--text-secondary)',
              cursor: 'pointer',
              flexShrink: 0,
            }}
            title="Start New Chat"
          >
            <Plus size={12} />
            <span>New Chat</span>
          </button>

          <span
            className="truncate"
            style={{
              fontSize: '13px',
              fontWeight: 500,
              color: 'var(--text-primary)',
              maxWidth: '360px',
            }}
          >
            {task.title}
          </span>
        </div>

        {/* Right Info: Status & Active Model */}
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px', flexShrink: 0 }}>
          <span
            style={{
              fontSize: '11px',
              display: 'inline-flex',
              alignItems: 'center',
              gap: '5px',
              padding: '2px 8px',
              borderRadius: 'var(--radius-sm)',
              backgroundColor: isGenerating ? 'var(--accent-subtle)' : 'var(--bg-panel)',
              border: `1px solid ${isGenerating ? 'var(--accent-border)' : 'var(--border-subtle)'}`,
              color: isGenerating ? 'var(--accent-primary)' : 'var(--text-secondary)',
              fontFamily: 'var(--font-mono)',
            }}
          >
            <span
              style={{
                width: '6px',
                height: '6px',
                borderRadius: '50%',
                backgroundColor: isGenerating ? 'var(--accent-primary)' : 'var(--status-success)',
              }}
            />
            {isGenerating ? 'Streaming...' : 'Ready'}
          </span>

          <span
            style={{
              fontSize: '11px',
              color: 'var(--accent-primary)',
              padding: '2px 6px',
              borderRadius: 'var(--radius-sm)',
              backgroundColor: 'var(--bg-panel)',
              border: '1px solid var(--border-subtle)',
              fontFamily: 'var(--font-mono)',
            }}
          >
            {task.model || '@cf/qwen/qwen3.8-27b'}
          </span>
        </div>
      </div>

      {/* Messages Scroll Area */}
      <div
        ref={scrollContainerRef}
        onScroll={handleScroll}
        style={{
          flex: 1,
          overflowY: 'auto',
          padding: '20px 24px',
          display: 'flex',
          flexDirection: 'column',
          gap: '16px',
        }}
      >
        <div style={{ maxWidth: '800px', width: '100%', margin: '0 auto', display: 'flex', flexDirection: 'column', gap: '18px' }}>
          {messages.map((msg, index) => {
            const isUser = msg.sender === 'user';

            if (isUser) {
              return (
                /* Distinct User Message (Aligned Right) */
                <div
                  key={msg.id || index}
                  style={{
                    display: 'flex',
                    flexDirection: 'column',
                    alignItems: 'flex-end',
                    width: '100%',
                  }}
                >
                  <div
                    style={{
                      maxWidth: '75%',
                      backgroundColor: '#262831',
                      border: '1px solid #383A44',
                      borderRadius: '10px 10px 2px 10px',
                      padding: '10px 14px',
                      color: 'var(--text-primary)',
                      fontSize: '13px',
                      lineHeight: 1.5,
                      whiteSpace: 'pre-wrap',
                      wordBreak: 'break-word',
                    }}
                  >
                    {msg.text}
                  </div>
                  {msg.timestamp && (
                    <span style={{ fontSize: '10px', color: 'var(--text-muted)', marginTop: '4px', marginRight: '2px' }}>
                      {msg.timestamp}
                    </span>
                  )}
                </div>
              );
            }

            /* Assistant (Aven) Response (Aligned Left) */
            return (
              <div
                key={msg.id || index}
                style={{
                  display: 'flex',
                  alignItems: 'flex-start',
                  gap: '10px',
                  width: '100%',
                  maxWidth: '92%',
                }}
              >
                {/* Aven Avatar */}
                <img
                  src="/aven.svg"
                  alt="Aven"
                  style={{
                    width: '20px',
                    height: '20px',
                    borderRadius: '4px',
                    objectFit: 'contain',
                    flexShrink: 0,
                    marginTop: '4px',
                  }}
                />

                <div style={{ flex: 1, display: 'flex', flexDirection: 'column' }}>
                  {/* Container */}
                  <div
                    style={{
                      backgroundColor: 'var(--bg-panel)',
                      border: '1px solid var(--border-subtle)',
                      borderRadius: '4px 10px 10px 10px',
                      padding: '12px 16px',
                    }}
                  >
                    {/* Header */}
                    <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '8px' }}>
                      <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                        <span style={{ fontSize: '12px', fontWeight: 600, color: 'var(--text-primary)' }}>
                          Aven
                        </span>
                        <span style={{ fontSize: '10px', color: 'var(--accent-primary)', fontFamily: 'var(--font-mono)' }}>
                          {task.model || '@cf/qwen/qwen3.8-27b'}
                        </span>
                      </div>

                      <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                        {msg.text && (
                          <button
                            type="button"
                            onClick={() => handleCopyText(msg.text, index)}
                            style={{
                              background: 'transparent',
                              border: 'none',
                              color: 'var(--text-muted)',
                              cursor: 'pointer',
                              display: 'flex',
                              alignItems: 'center',
                              gap: '4px',
                              fontSize: '11px',
                              padding: '2px 4px',
                            }}
                            title="Copy response"
                          >
                            {copiedIndex === index ? (
                              <CheckCheck size={12} color="var(--status-success)" />
                            ) : (
                              <Copy size={12} />
                            )}
                            <span>{copiedIndex === index ? 'Copied' : 'Copy'}</span>
                          </button>
                        )}
                        {msg.timestamp && (
                          <span style={{ fontSize: '10px', color: 'var(--text-muted)' }}>
                            {msg.timestamp}
                          </span>
                        )}
                      </div>
                    </div>

                    {/* Genuine Initial Loading indicator if text is still empty */}
                    {msg.status === 'thinking' && !msg.text && (
                      <div style={{ display: 'flex', flexDirection: 'column', gap: '8px', padding: '6px 0' }}>
                        <div style={{ display: 'flex', alignItems: 'center', gap: '8px', color: 'var(--accent-primary)', fontSize: '12px' }}>
                          <span
                            style={{
                              width: '8px',
                              height: '8px',
                              borderRadius: '50%',
                              backgroundColor: 'var(--accent-primary)',
                              boxShadow: '0 0 8px var(--accent-primary)',
                            }}
                            className="animate-pulse"
                          />
                          <span>Connecting to Cloudflare Workers AI...</span>
                        </div>
                      </div>
                    )}

                    {/* Markdown Formatted Content with Incremental Streaming Support */}
                    {msg.text && (
                      <MarkdownMessage
                        content={msg.text}
                        isStreaming={msg.status === 'thinking'}
                      />
                    )}

                    {/* Error State with Sanitized Reason & Explicit Retry */}
                    {msg.status === 'error' && (
                      <div style={{ marginTop: '10px', display: 'flex', flexDirection: 'column', gap: '8px' }}>
                        <div
                          style={{
                            padding: '8px 12px',
                            borderRadius: 'var(--radius-sm)',
                            backgroundColor: 'rgba(239, 68, 68, 0.1)',
                            border: '1px solid rgba(239, 68, 68, 0.25)',
                            display: 'flex',
                            alignItems: 'flex-start',
                            gap: '8px',
                          }}
                        >
                          <AlertCircle size={15} color="var(--status-danger)" style={{ flexShrink: 0, marginTop: '2px' }} />
                          <div style={{ flex: 1, fontSize: '12px', color: 'var(--status-danger)' }}>
                            <div style={{ fontWeight: 600 }}>Request Failed or Interrupted</div>
                            {msg.logs && msg.logs.length > 0 && (
                              <div style={{ marginTop: '4px', fontSize: '11px', color: 'var(--text-secondary)', whiteSpace: 'pre-line' }}>
                                {msg.logs[msg.logs.length - 1].message}
                              </div>
                            )}
                          </div>
                        </div>

                        <div>
                          <button
                            type="button"
                            onClick={() => taskStore.retryMessage(task.id, msg.id)}
                            style={{
                              display: 'inline-flex',
                              alignItems: 'center',
                              gap: '6px',
                              padding: '5px 10px',
                              borderRadius: 'var(--radius-sm)',
                              backgroundColor: 'var(--bg-main)',
                              border: '1px solid var(--border-subtle)',
                              color: 'var(--text-primary)',
                              fontSize: '11.5px',
                              cursor: 'pointer',
                            }}
                          >
                            <RotateCcw size={12} color="var(--accent-primary)" />
                            <span>Retry Request</span>
                          </button>
                        </div>
                      </div>
                    )}

                    {/* Token Metrics Footer */}
                    {msg.status === 'done' && task.tokenUsage && (
                      <div
                        style={{
                          display: 'flex',
                          alignItems: 'center',
                          gap: '12px',
                          marginTop: '10px',
                          paddingTop: '8px',
                          borderTop: '1px solid var(--border-subtle-dim)',
                          fontSize: '11px',
                          color: 'var(--text-muted)',
                          fontFamily: 'var(--font-mono)',
                        }}
                      >
                        <span>{task.tokenUsage.totalTokens.toLocaleString()} tokens</span>
                        {task.duration && (
                          <>
                            <span>•</span>
                            <span>{task.duration}</span>
                          </>
                        )}
                      </div>
                    )}
                  </div>
                </div>
              </div>
            );
          })}
          <div ref={messagesEndRef} />
        </div>
      </div>

      {/* Bottom Sticky Composer */}
      <div
        style={{
          borderTop: '1px solid var(--border-subtle)',
          backgroundColor: 'var(--bg-main)',
          padding: '12px 24px',
          flexShrink: 0,
        }}
      >
        <div style={{ maxWidth: '800px', margin: '0 auto', width: '100%' }}>
          <div
            style={{
              backgroundColor: 'var(--bg-panel)',
              border: '1px solid var(--border-subtle)',
              borderRadius: 'var(--radius-lg)',
              padding: '10px 12px 8px 12px',
              transition: 'border-color var(--transition-fast)',
            }}
          >
            {/* Multi-line Textarea */}
            <textarea
              ref={textareaRef}
              value={replyText}
              onChange={(e) => setReplyText(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder="Ask Cloudflare Workers AI... (Enter to send, Shift+Enter for new line)"
              rows={2}
              style={{
                width: '100%',
                backgroundColor: 'transparent',
                border: 'none',
                outline: 'none',
                color: 'var(--text-primary)',
                fontSize: '13px',
                lineHeight: 1.5,
                resize: 'none',
                padding: 0,
              }}
            />

            {/* Bottom Row of Composer */}
            <div
              style={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'space-between',
                paddingTop: '6px',
                marginTop: '4px',
                borderTop: '1px solid var(--border-subtle-dim)',
              }}
            >
              {/* Model Picker */}
              <div style={{ position: 'relative' }}>
                <button
                  type="button"
                  onClick={() => setShowModelDropdown(!showModelDropdown)}
                  style={{
                    display: 'inline-flex',
                    alignItems: 'center',
                    gap: '5px',
                    padding: '3px 7px',
                    borderRadius: 'var(--radius-sm)',
                    backgroundColor: 'var(--bg-input)',
                    border: '1px solid var(--border-subtle)',
                    fontSize: '11px',
                    color: 'var(--text-secondary)',
                    cursor: 'pointer',
                  }}
                >
                  <Sparkles size={11} color="var(--accent-primary)" />
                  <span style={{ fontWeight: 500, color: 'var(--text-primary)', fontFamily: 'var(--font-mono)' }}>
                    {selectedModel}
                  </span>
                  <ChevronDown size={11} color="var(--text-muted)" />
                </button>

                {showModelDropdown && (
                  <div
                    style={{
                      position: 'absolute',
                      bottom: '28px',
                      left: 0,
                      width: '240px',
                      backgroundColor: 'var(--bg-dropdown)',
                      border: '1px solid var(--border-subtle)',
                      borderRadius: 'var(--radius-md)',
                      padding: '4px',
                      boxShadow: '0 8px 24px rgba(0,0,0,0.6)',
                      zIndex: 50,
                    }}
                  >
                    {availableModels.map((m) => {
                      const isSelected = m === selectedModel;
                      return (
                        <button
                          key={m}
                          type="button"
                          onClick={() => {
                            setSelectedModel(m);
                            setShowModelDropdown(false);
                          }}
                          style={{
                            width: '100%',
                            padding: '5px 8px',
                            borderRadius: 'var(--radius-sm)',
                            backgroundColor: isSelected ? 'var(--accent-subtle)' : 'transparent',
                            color: isSelected ? 'var(--text-accent)' : 'var(--text-secondary)',
                            fontSize: '11px',
                            fontFamily: 'var(--font-mono)',
                            display: 'flex',
                            alignItems: 'center',
                            justifyContent: 'space-between',
                            cursor: 'pointer',
                            textAlign: 'left',
                          }}
                        >
                          <span>{m}</span>
                          {isSelected && <Check size={12} color="var(--accent-primary)" />}
                        </button>
                      );
                    })}
                  </div>
                )}
              </div>

              {/* Action Buttons: Stop Generating or Send */}
              <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                {isGenerating ? (
                  <Button
                    variant="secondary"
                    size="sm"
                    onClick={() => taskStore.stopGenerating(task.id)}
                    icon={<Square size={12} fill="currentColor" />}
                    style={{
                      color: 'var(--status-danger)',
                      borderColor: 'rgba(239, 68, 68, 0.4)',
                      backgroundColor: 'rgba(239, 68, 68, 0.1)',
                      gap: '5px',
                    }}
                    title="Stop Generating"
                  >
                    Stop Generating
                  </Button>
                ) : (
                  <>
                    <span style={{ fontSize: '11px', color: 'var(--text-muted)' }}>
                      Enter to send
                    </span>
                    <Button
                      variant="primary"
                      size="sm"
                      onClick={() => handleSend()}
                      disabled={!replyText.trim()}
                      icon={<ArrowUp size={13} />}
                      style={{ width: '28px', height: '28px', padding: 0 }}
                      title="Send message"
                    />
                  </>
                )}
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
