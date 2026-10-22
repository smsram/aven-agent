import React, { useState, useRef, useEffect } from 'react';
import { ArrowUp, Sparkles, ChevronDown, Check, Plus, Copy, CheckCheck } from 'lucide-react';
import { AgentTask } from '../../types';
import { Button } from '../../components/common/Button';

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
  const [selectedModel, setSelectedModel] = useState(task.model || 'Claude 3.7 Sonnet');
  const [showModelDropdown, setShowModelDropdown] = useState(false);
  const [copiedIndex, setCopiedIndex] = useState<number | null>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const isGenerating = task.status === 'running' || task.messages?.some((m) => m.status === 'thinking');

  // Auto-scroll to bottom on new messages or logs
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [task.messages, task.logs]);

  // Focus textarea on mount
  useEffect(() => {
    textareaRef.current?.focus();
  }, [task.id]);

  const handleSend = (e?: React.FormEvent) => {
    if (e) e.preventDefault();
    if (!replyText.trim() || isGenerating) return;
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
            {isGenerating ? 'Aven Working...' : 'Ready'}
          </span>

          <span
            style={{
              fontSize: '11px',
              color: 'var(--text-muted)',
              padding: '2px 6px',
              borderRadius: 'var(--radius-sm)',
              backgroundColor: 'var(--bg-panel)',
              border: '1px solid var(--border-subtle)',
            }}
          >
            {task.model}
          </span>
        </div>
      </div>

      {/* Messages Scroll Area */}
      <div
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
                  maxWidth: '88%',
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
                    marginTop: '2px',
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
                        <span style={{ fontSize: '10px', color: 'var(--accent-primary)', fontWeight: 500 }}>
                          {task.model}
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

                    {/* Genuine Loading / Thinking State */}
                    {msg.status === 'thinking' && (
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
                          />
                          <span>Aven is analyzing workspace context...</span>
                        </div>

                        {/* Step Logs Stream */}
                        {msg.logs && msg.logs.length > 0 && (
                          <div
                            style={{
                              marginTop: '6px',
                              backgroundColor: 'var(--bg-main)',
                              border: '1px solid var(--border-subtle)',
                              borderRadius: 'var(--radius-sm)',
                              padding: '8px 10px',
                              fontFamily: 'var(--font-mono)',
                              fontSize: '11px',
                              display: 'flex',
                              flexDirection: 'column',
                              gap: '4px',
                            }}
                          >
                            {msg.logs.map((log, li) => (
                              <div key={li} style={{ display: 'flex', gap: '6px', color: 'var(--text-secondary)' }}>
                                <span style={{ color: 'var(--text-muted)' }}>{log.timestamp}</span>
                                <span style={{ color: 'var(--accent-primary)' }}>•</span>
                                <span>{log.message}</span>
                              </div>
                            ))}
                          </div>
                        )}
                      </div>
                    )}

                    {/* Result Content */}
                    {msg.status !== 'thinking' && msg.text && (
                      <div
                        style={{
                          fontSize: '13px',
                          color: 'var(--text-primary)',
                          lineHeight: 1.55,
                          whiteSpace: 'pre-wrap',
                          wordBreak: 'break-word',
                        }}
                      >
                        {msg.text}
                      </div>
                    )}

                    {/* Error State if any */}
                    {msg.status === 'error' && (
                      <div style={{ color: 'var(--status-danger)', fontSize: '12px', marginTop: '6px' }}>
                        Execution encountered an issue. Please verify your connection or model settings.
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
              placeholder="Reply to Aven... (Enter to send, Shift+Enter for new line)"
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
                  <span style={{ fontWeight: 500, color: 'var(--text-primary)' }}>{selectedModel}</span>
                  <ChevronDown size={11} color="var(--text-muted)" />
                </button>

                {showModelDropdown && (
                  <div
                    style={{
                      position: 'absolute',
                      bottom: '28px',
                      left: 0,
                      width: '200px',
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
                            fontSize: '12px',
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

              {/* Send Button & Hint */}
              <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                <span style={{ fontSize: '11px', color: 'var(--text-muted)' }}>
                  Enter to send
                </span>
                <Button
                  variant="primary"
                  size="sm"
                  onClick={() => handleSend()}
                  disabled={!replyText.trim() || isGenerating}
                  icon={<ArrowUp size={13} />}
                  style={{ width: '28px', height: '28px', padding: 0 }}
                  title="Send message"
                />
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
