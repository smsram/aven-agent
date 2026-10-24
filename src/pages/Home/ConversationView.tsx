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
  Info,
  X,
} from 'lucide-react';
import { AgentTask, ProcessStep, MessageMetadata } from '../../types';
import { Button } from '../../components/common/Button';
import { MarkdownMessage } from '../../components/chat/MarkdownMessage';
import { IntegrationIcon } from '../../components/icons/IntegrationIcon';
import { ErrorBoundary } from '../../components/common/ErrorBoundary';
import { taskStore } from '../../services/taskStore';
import {
  getModelDisplayName,
  DEFAULT_MODEL_ID,
} from '../../services/modelRegistry';

export interface ConversationViewProps {
  task: AgentTask;
  onSendMessage: (text: string, model: string) => void;
  onNewChat: () => void;
  availableModels: string[];
  onModelChange?: (model: string) => void;
}

/**
 * Truncates long user messages (> ~6-8 lines) with a smooth Show more / Show less toggle
 * without mutating or modifying the underlying message content.
 */
const UserMessageBubble: React.FC<{ text: string; timestamp?: string }> = ({ text, timestamp }) => {
  const [isExpanded, setIsExpanded] = useState(false);
  const [isOverflowing, setIsOverflowing] = useState(false);
  const [copied, setCopied] = useState(false);
  const contentRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (contentRef.current) {
      // Check if rendered height exceeds approximately 7 lines (~150px)
      setIsOverflowing(contentRef.current.scrollHeight > 150);
    }
  }, [text]);

  const handleCopy = (e: React.MouseEvent) => {
    e.stopPropagation();
    navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'flex-end',
        width: '100%',
      }}
    >
      <div
        style={{
          maxWidth: '80%',
          backgroundColor: '#262831',
          border: '1px solid #383A44',
          borderRadius: '12px 12px 3px 12px',
          padding: '10px 14px',
          color: 'var(--text-primary)',
          fontSize: '13px',
          lineHeight: 1.5,
          position: 'relative',
        }}
      >
        <div
          ref={contentRef}
          style={{
            maxHeight: !isOverflowing || isExpanded ? 'none' : '135px',
            overflow: 'hidden',
            whiteSpace: 'pre-wrap',
            wordBreak: 'break-word',
            overflowWrap: 'anywhere',
            position: 'relative',
          }}
        >
          {text}
          {isOverflowing && !isExpanded && (
            <div
              style={{
                position: 'absolute',
                bottom: 0,
                left: 0,
                right: 0,
                height: '36px',
                background: 'linear-gradient(to bottom, transparent, #262831)',
                pointerEvents: 'none',
              }}
            />
          )}
        </div>

        {isOverflowing && (
          <button
            type="button"
            onClick={() => setIsExpanded(!isExpanded)}
            style={{
              marginTop: '6px',
              padding: '2px 0',
              fontSize: '11px',
              color: 'var(--accent-primary)',
              background: 'transparent',
              border: 'none',
              cursor: 'pointer',
              fontWeight: 500,
            }}
          >
            {isExpanded ? 'Show less' : 'Show more'}
          </button>
        )}
      </div>

      {/* Timestamp and Subtle Copy Control */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: '6px',
          marginTop: '4px',
          marginRight: '4px',
          color: 'var(--text-muted)',
          fontSize: '10px',
        }}
      >
        <button
          type="button"
          onClick={handleCopy}
          style={{
            background: 'transparent',
            border: 'none',
            color: copied ? 'var(--status-success)' : 'var(--text-muted)',
            cursor: 'pointer',
            display: 'inline-flex',
            alignItems: 'center',
            gap: '3px',
            fontSize: '10px',
            padding: '2px',
            borderRadius: 'var(--radius-sm)',
            transition: 'color var(--transition-fast)',
          }}
          onMouseEnter={(e) => {
            if (!copied) e.currentTarget.style.color = 'var(--text-secondary)';
          }}
          onMouseLeave={(e) => {
            if (!copied) e.currentTarget.style.color = 'var(--text-muted)';
          }}
          title={copied ? 'Copied' : 'Copy message'}
          aria-label={copied ? 'Copied user message' : 'Copy user message'}
        >
          {copied ? (
            <>
              <CheckCheck size={11} color="var(--status-success)" />
              <span style={{ color: 'var(--status-success)', fontSize: '10px' }}>Copied</span>
            </>
          ) : (
            <Copy size={11} />
          )}
        </button>

        {timestamp && (
          <span style={{ fontSize: '10px', color: 'var(--text-muted)' }}>
            {timestamp}
          </span>
        )}
      </div>
    </div>
  );
};


/**
 * Compact, persistent, expandable activity bar with real integration logos,
 * left-to-right shimmering active text, and a loading indicator while generating.
 */
const ProcessActivityBar: React.FC<{
  steps: ProcessStep[];
  activeStatus?: { status: string; description: string; integration_id?: string; tool_name?: string };
  isGenerating: boolean;
}> = ({ steps, activeStatus, isGenerating }) => {
  const [isExpanded, setIsExpanded] = useState(false);
  const [isHovered, setIsHovered] = useState(false);
  const [isFocused, setIsFocused] = useState(false);

  // If no steps exist and not generating and no active status, omit
  if (steps.length === 0 && !activeStatus && !isGenerating) {
    return null;
  }

  const activeStep = steps.find((s) => s.status === 'running');
  const hasFailed = steps.some((s) => s.status === 'failed');
  const completedCount = steps.filter((s) => s.status === 'completed').length;

  let summaryLabel = 'Working';
  let activeIntegrationId = activeStatus?.integration_id;

  if (activeStatus?.description) {
    summaryLabel = activeStatus.description;
    activeIntegrationId = activeStatus.integration_id || activeStatus.tool_name;
  } else if (activeStep) {
    summaryLabel = activeStep.label;
    activeIntegrationId = activeStep.integration_id || activeStep.tool_name;
  } else if (isGenerating) {
    summaryLabel = 'Generating response';
  } else if (hasFailed) {
    summaryLabel = `Activity failed (${steps.length} steps)`;
  } else if (steps.length > 0) {
    summaryLabel = `Completed ${completedCount} ${completedCount === 1 ? 'step' : 'steps'}`;
  }

  const showChevron = isHovered || isFocused || isExpanded;

  return (
    <div
      style={{
        marginBottom: '10px',
        maxWidth: '100%',
      }}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
    >
      {/* Clickable compact summary pill */}
      <button
        type="button"
        onClick={() => setIsExpanded(!isExpanded)}
        onFocus={() => setIsFocused(true)}
        onBlur={() => setIsFocused(false)}
        aria-expanded={isExpanded}
        aria-label={`Process activity: ${summaryLabel}. Click to ${isExpanded ? 'collapse' : 'expand'} details.`}
        style={{
          display: 'inline-flex',
          alignItems: 'center',
          gap: '7px',
          padding: '4px 8px',
          borderRadius: 'var(--radius-sm)',
          backgroundColor: 'rgba(255, 255, 255, 0.03)',
          border: '1px solid var(--border-subtle)',
          cursor: 'pointer',
          color: 'var(--text-secondary)',
          fontSize: '11.5px',
          textAlign: 'left',
          transition: 'background-color 150ms ease, border-color 150ms ease',
        }}
      >
        {/* Polished loading spinner only while generating */}
        {isGenerating && (
          <span
            className="aven-spinner"
            style={{
              width: '12px',
              height: '12px',
              border: '1.5px solid rgba(124, 124, 248, 0.3)',
              borderTopColor: 'var(--accent-primary)',
              borderRadius: '50%',
              display: 'inline-block',
              flexShrink: 0,
            }}
          />
        )}

        {/* Integration Logo if active */}
        {activeIntegrationId && (
          <IntegrationIcon integrationId={activeIntegrationId} size={13} />
        )}

        {/* Left-to-right shimmering text for running state; static otherwise */}
        <span
          className={isGenerating ? 'aven-shimmer-text' : ''}
          style={{
            fontWeight: 450,
            color: isGenerating ? undefined : 'var(--text-secondary)',
          }}
        >
          {summaryLabel}
        </span>

        {/* Expand/Collapse Chevron (visible on hover, focus, or when expanded) */}
        <span
          style={{
            opacity: showChevron ? 1 : 0,
            transition: 'opacity 150ms ease, transform 150ms ease',
            transform: isExpanded ? 'rotate(180deg)' : 'none',
            display: 'inline-flex',
            alignItems: 'center',
            color: 'var(--text-muted)',
            marginLeft: '2px',
          }}
        >
          <ChevronDown size={12} />
        </span>
      </button>

      {/* Expanded Activity Timeline Details */}
      {isExpanded && (
        <div
          style={{
            marginTop: '6px',
            padding: '8px 12px',
            borderRadius: 'var(--radius-sm)',
            backgroundColor: '#1E1F25',
            border: '1px solid var(--border-subtle)',
            maxWidth: '540px',
            display: 'flex',
            flexDirection: 'column',
            gap: '6px',
          }}
        >
          {steps.map((step) => {
            const isStepRunning = step.status === 'running';
            const isStepCompleted = step.status === 'completed';
            const isStepFailed = step.status === 'failed';

            return (
              <div
                key={step.id}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: '8px',
                  fontSize: '11px',
                  color: isStepRunning ? 'var(--text-primary)' : 'var(--text-secondary)',
                }}
              >
                {/* Status Indicator */}
                <div style={{ width: '14px', height: '14px', display: 'flex', alignItems: 'center', justifyContent: 'center', flexShrink: 0 }}>
                  {isStepRunning ? (
                    <span
                      className="aven-spinner"
                      style={{
                        width: '10px',
                        height: '10px',
                        border: '1.5px solid rgba(124, 124, 248, 0.3)',
                        borderTopColor: 'var(--accent-primary)',
                        borderRadius: '50%',
                        display: 'inline-block',
                      }}
                    />
                  ) : isStepCompleted ? (
                    <Check size={11} color="var(--status-success)" />
                  ) : isStepFailed ? (
                    <span style={{ width: '6px', height: '6px', borderRadius: '50%', backgroundColor: 'var(--status-danger)' }} />
                  ) : (
                    <span style={{ width: '6px', height: '6px', borderRadius: '50%', backgroundColor: 'var(--text-muted)' }} />
                  )}
                </div>

                {/* Integration Logo */}
                <IntegrationIcon integrationId={step.integration_id || step.tool_name} size={13} />

                {/* Step Description */}
                <span style={{ flex: 1, wordBreak: 'break-word' }}>
                  {step.label}
                </span>

                {/* Timestamp */}
                {step.timestamp && (
                  <span style={{ fontSize: '9.5px', color: 'var(--text-muted)', fontFamily: 'var(--font-mono)' }}>
                    {step.timestamp}
                  </span>
                )}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
};

/**
 * Subtle details popover control showing actual measured model and token usage.
 */
const MessageMetadataPopover: React.FC<{
  metadata?: MessageMetadata;
  fallbackModel?: string;
}> = ({ metadata, fallbackModel }) => {
  const [isOpen, setIsOpen] = useState(false);
  const popoverRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!isOpen) return;
    const handleClickOutside = (e: MouseEvent) => {
      if (popoverRef.current && !popoverRef.current.contains(e.target as Node)) {
        setIsOpen(false);
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, [isOpen]);

  const model = metadata?.model || fallbackModel || DEFAULT_MODEL_ID;
  const duration = metadata?.duration_formatted || (metadata?.duration_ms ? `${(metadata.duration_ms / 1000).toFixed(1)}s` : undefined);
  const promptTokens = metadata?.prompt_tokens;
  const completionTokens = metadata?.completion_tokens;
  const totalTokens = metadata?.total_tokens;

  return (
    <div ref={popoverRef} style={{ position: 'relative', display: 'inline-flex' }}>
      <button
        type="button"
        onClick={() => setIsOpen(!isOpen)}
        style={{
          background: 'transparent',
          border: 'none',
          color: isOpen ? 'var(--text-primary)' : 'var(--text-muted)',
          cursor: 'pointer',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          padding: '2px 4px',
          borderRadius: 'var(--radius-sm)',
        }}
        title="Response details"
        aria-label="Response details"
      >
        <Info size={12} />
      </button>

      {isOpen && (
        <div
          style={{
            position: 'absolute',
            bottom: '26px',
            left: 0,
            width: '210px',
            backgroundColor: 'var(--bg-dropdown)',
            border: '1px solid var(--border-subtle)',
            borderRadius: 'var(--radius-md)',
            padding: '10px 12px',
            boxShadow: '0 8px 24px rgba(0,0,0,0.6)',
            zIndex: 60,
            fontSize: '11px',
            color: 'var(--text-secondary)',
            display: 'flex',
            flexDirection: 'column',
            gap: '6px',
          }}
        >
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', borderBottom: '1px solid var(--border-subtle-dim)', paddingBottom: '5px', marginBottom: '2px' }}>
            <span style={{ fontWeight: 600, color: 'var(--text-primary)', fontSize: '11.5px' }}>Response details</span>
            <button
              type="button"
              onClick={() => setIsOpen(false)}
              style={{ background: 'transparent', border: 'none', color: 'var(--text-muted)', cursor: 'pointer', padding: 0 }}
            >
              <X size={11} />
            </button>
          </div>

          <div style={{ display: 'flex', justifyContent: 'space-between' }}>
            <span style={{ color: 'var(--text-muted)' }}>Model</span>
            <span style={{ fontFamily: 'var(--font-mono)', color: 'var(--text-primary)' }} title={model}>
              {getModelDisplayName(model)}
            </span>
          </div>

          {duration && (
            <div style={{ display: 'flex', justifyContent: 'space-between' }}>
              <span style={{ color: 'var(--text-muted)' }}>Duration</span>
              <span style={{ fontFamily: 'var(--font-mono)', color: 'var(--text-primary)' }}>{duration}</span>
            </div>
          )}

          {promptTokens !== undefined && (
            <div style={{ display: 'flex', justifyContent: 'space-between' }}>
              <span style={{ color: 'var(--text-muted)' }}>Input tokens</span>
              <span style={{ fontFamily: 'var(--font-mono)', color: 'var(--text-primary)' }}>{promptTokens.toLocaleString()}</span>
            </div>
          )}

          {completionTokens !== undefined && (
            <div style={{ display: 'flex', justifyContent: 'space-between' }}>
              <span style={{ color: 'var(--text-muted)' }}>Output tokens</span>
              <span style={{ fontFamily: 'var(--font-mono)', color: 'var(--text-primary)' }}>{completionTokens.toLocaleString()}</span>
            </div>
          )}

          {totalTokens !== undefined && (
            <div style={{ display: 'flex', justifyContent: 'space-between', borderTop: '1px solid var(--border-subtle-dim)', paddingTop: '4px', marginTop: '2px' }}>
              <span style={{ fontWeight: 500, color: 'var(--text-primary)' }}>Total tokens</span>
              <span style={{ fontFamily: 'var(--font-mono)', fontWeight: 600, color: 'var(--text-primary)' }}>{totalTokens.toLocaleString()}</span>
            </div>
          )}

          {!duration && promptTokens === undefined && completionTokens === undefined && totalTokens === undefined && (
            <div style={{ color: 'var(--text-muted)', fontSize: '10.5px', fontStyle: 'italic' }}>
              Token counts unavailable for this response.
            </div>
          )}
        </div>
      )}
    </div>
  );
};

export const ConversationView: React.FC<ConversationViewProps> = ({
  task,
  onSendMessage,
  onNewChat,
  availableModels,
  onModelChange,
}) => {
  const [replyText, setReplyText] = useState('');
  const [selectedModel, setSelectedModel] = useState(task.model || DEFAULT_MODEL_ID);
  const [showModelDropdown, setShowModelDropdown] = useState(false);
  const [copiedIndex, setCopiedIndex] = useState<number | null>(null);

  useEffect(() => {
    if (task.model) {
      setSelectedModel(task.model);
    }
  }, [task.id, task.model]);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const scrollContainerRef = useRef<HTMLDivElement>(null);
  const isUserScrolledUp = useRef(false);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const isGenerating = task.status === 'running' || task.messages?.some((m) => m.status === 'thinking');

  const handleScroll = () => {
    if (!scrollContainerRef.current) return;
    const { scrollTop, scrollHeight, clientHeight } = scrollContainerRef.current;
    isUserScrolledUp.current = scrollHeight - scrollTop - clientHeight > 80;
  };

  useEffect(() => {
    if (!isUserScrolledUp.current) {
      messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
    }
  }, [task.messages, task.logs]);

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
      {/* Conversation Top Header Bar: Clean Title & Controls only (No "Ready" or Model badge) */}
      <div
        style={{
          height: '42px',
          padding: '0 24px',
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
              maxWidth: '460px',
            }}
          >
            {task.title}
          </span>
        </div>
      </div>

      {/* Messages Scroll Area: Responsive Content Width (up to 1000px) */}
      <div
        ref={scrollContainerRef}
        onScroll={handleScroll}
        style={{
          flex: 1,
          overflowY: 'auto',
          padding: '24px 32px',
          display: 'flex',
          flexDirection: 'column',
          gap: '20px',
        }}
      >
        <div
          style={{
            maxWidth: '1000px',
            width: '100%',
            margin: '0 auto',
            display: 'flex',
            flexDirection: 'column',
            gap: '22px',
          }}
        >
          {messages.map((msg, index) => {
            const isUser = msg.sender === 'user';

            if (isUser) {
              return (
                <UserMessageBubble
                  key={msg.id || index}
                  text={msg.text}
                  timestamp={msg.timestamp}
                />
              );
            }

            const steps = msg.processSteps || msg.process_steps || [];
            const isMsgThinking = msg.status === 'thinking';

            /* Assistant Response: Rendered directly on the chat surface (NO enclosing card background) */
            return (
              <div
                key={msg.id || index}
                style={{
                  display: 'flex',
                  flexDirection: 'column',
                  width: '100%',
                  alignItems: 'flex-start',
                }}
              >
                {/* Persistent Expandable Activity / Process Steps Indicator */}
                <ProcessActivityBar
                  steps={steps}
                  activeStatus={msg.activityStatus}
                  isGenerating={isMsgThinking}
                />

                {/* Markdown Formatted Content rendered directly on chat background */}
                {msg.text && (
                  <div style={{ width: '100%', minWidth: 0 }}>
                    <ErrorBoundary
                      sectionName="Message formatting"
                      fallback={
                        <div style={{ whiteSpace: 'pre-wrap', wordBreak: 'break-word', color: 'var(--text-primary)', fontSize: '13px', lineHeight: 1.5 }}>
                          {msg.text}
                        </div>
                      }
                    >
                      <MarkdownMessage
                        content={msg.text}
                        isStreaming={isMsgThinking}
                      />
                    </ErrorBoundary>
                  </div>
                )}

                {/* Initial Loading placeholder if text is still empty and thinking */}
                {isMsgThinking && !msg.text && !msg.activityStatus && (
                  <div style={{ display: 'flex', alignItems: 'center', gap: '8px', padding: '6px 0', color: 'var(--text-secondary)', fontSize: '12px' }}>
                    <span
                      className="aven-spinner"
                      style={{
                        width: '12px',
                        height: '12px',
                        border: '1.5px solid rgba(124, 124, 248, 0.3)',
                        borderTopColor: 'var(--accent-primary)',
                        borderRadius: '50%',
                        display: 'inline-block',
                      }}
                    />
                    <span className="aven-shimmer-text">Connecting to Cloudflare Workers AI...</span>
                  </div>
                )}

                {/* Error State with Sanitized Reason & Explicit Retry */}
                {msg.status === 'error' && (
                  <div style={{ marginTop: '10px', width: '100%', display: 'flex', flexDirection: 'column', gap: '8px' }}>
                    <div
                      style={{
                        padding: '8px 12px',
                        borderRadius: 'var(--radius-sm)',
                        backgroundColor: 'rgba(239, 68, 68, 0.08)',
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
                          backgroundColor: 'var(--bg-panel)',
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

                {/* Action Bar (Copy Button and Info Details Popover) - Subtly aligned */}
                {!isMsgThinking && (msg.text || msg.status === 'done') && (
                  <div
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      gap: '8px',
                      marginTop: '8px',
                      color: 'var(--text-muted)',
                      fontSize: '11px',
                    }}
                  >
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

                    {/* Subtle details control for model & token metadata */}
                    <MessageMetadataPopover
                      metadata={msg.metadata}
                      fallbackModel={task.model}
                    />

                    {msg.timestamp && (
                      <span style={{ fontSize: '10px', color: 'var(--text-muted)', marginLeft: '4px' }}>
                        {msg.timestamp}
                      </span>
                    )}
                  </div>
                )}
              </div>
            );
          })}
          <div ref={messagesEndRef} />
        </div>
      </div>

      {/* Bottom Sticky Composer: Polished, Compact, Clean styling */}
      <div
        style={{
          borderTop: '1px solid var(--border-subtle)',
          backgroundColor: 'var(--bg-main)',
          padding: '12px 32px',
          flexShrink: 0,
        }}
      >
        <div style={{ maxWidth: '1000px', margin: '0 auto', width: '100%' }}>
          <div
            style={{
              backgroundColor: 'var(--bg-panel)',
              border: '1px solid var(--border-subtle)',
              borderRadius: 'var(--radius-lg)',
              padding: '10px 12px 8px 12px',
              transition: 'border-color var(--transition-fast)',
            }}
          >
            {/* Multi-line Textarea with Natural Placeholder */}
            <textarea
              ref={textareaRef}
              value={replyText}
              onChange={(e) => setReplyText(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder="Ask Aven anything..."
              rows={2}
              aria-label="Message Aven"
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
                  title={selectedModel}
                >
                  <Sparkles size={11} color="var(--accent-primary)" />
                  <span style={{ fontWeight: 500, color: 'var(--text-primary)' }}>
                    {getModelDisplayName(selectedModel)}
                  </span>
                  <ChevronDown size={11} color="var(--text-muted)" />
                </button>

                {showModelDropdown && (
                  <div
                    style={{
                      position: 'absolute',
                      bottom: '28px',
                      left: 0,
                      width: '280px',
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
                      const displayName = getModelDisplayName(m);
                      return (
                        <button
                          key={m}
                          type="button"
                          onClick={() => {
                            setSelectedModel(m);
                            setShowModelDropdown(false);
                            onModelChange?.(m);
                          }}
                          style={{
                            width: '100%',
                            padding: '6px 8px',
                            borderRadius: 'var(--radius-sm)',
                            backgroundColor: isSelected ? 'var(--accent-subtle)' : 'transparent',
                            color: isSelected ? 'var(--text-accent)' : 'var(--text-primary)',
                            fontSize: '11px',
                            display: 'flex',
                            alignItems: 'center',
                            justifyContent: 'space-between',
                            cursor: 'pointer',
                            textAlign: 'left',
                            gap: '8px',
                          }}
                        >
                          <div style={{ display: 'flex', flexDirection: 'column', gap: '2px', minWidth: 0 }}>
                            <span style={{ fontWeight: isSelected ? 600 : 500 }}>{displayName}</span>
                            <span
                              style={{
                                fontSize: '9.5px',
                                color: 'var(--text-muted)',
                                fontFamily: 'var(--font-mono)',
                                overflow: 'hidden',
                                textOverflow: 'ellipsis',
                                whiteSpace: 'nowrap',
                              }}
                            >
                              {m}
                            </span>
                          </div>
                          {isSelected && <Check size={13} color="var(--accent-primary)" style={{ flexShrink: 0 }} />}
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
                    title="Stop agent · Ctrl+Shift+Backspace"
                  >
                    Stop agent · Ctrl+Shift+Backspace
                  </Button>
                ) : (
                  <Button
                    variant="primary"
                    size="sm"
                    onClick={() => handleSend()}
                    disabled={!replyText.trim()}
                    icon={<ArrowUp size={13} />}
                    style={{ width: '28px', height: '28px', padding: 0 }}
                    title="Send message"
                  />
                )}
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
