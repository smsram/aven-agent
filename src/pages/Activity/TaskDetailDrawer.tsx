import React from 'react';
import { X, FileCode, Copy, Check } from 'lucide-react';
import { AgentTask } from '../../types';
import { Badge } from '../../components/common/Badge';
import { Button } from '../../components/common/Button';

export interface TaskDetailDrawerProps {
  task: AgentTask | null;
  onClose: () => void;
}

export const TaskDetailDrawer: React.FC<TaskDetailDrawerProps> = ({ task, onClose }) => {
  const [copied, setCopied] = React.useState(false);

  if (!task) return null;

  const handleCopyOutput = () => {
    if (task.output) {
      navigator.clipboard.writeText(task.output);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  const getStatusBadge = () => {
    switch (task.status) {
      case 'completed':
        return <Badge variant="success" dot>Completed</Badge>;
      case 'running':
        return <Badge variant="indigo" dot>Running</Badge>;
      case 'failed':
        return <Badge variant="danger" dot>Failed</Badge>;
      default:
        return <Badge variant="muted" dot>Queued</Badge>;
    }
  };

  return (
    <div
      style={{
        position: 'fixed',
        top: 'var(--topbar-height)',
        right: 0,
        bottom: 0,
        width: '540px',
        maxWidth: '100vw',
        backgroundColor: 'var(--bg-panel)',
        borderLeft: '1px solid var(--border-subtle)',
        display: 'flex',
        flexDirection: 'column',
        zIndex: 50,
        boxShadow: '-8px 0 32px rgba(0,0,0,0.5)',
      }}
    >
      {/* Header */}
      <div
        style={{
          padding: 'var(--space-4) var(--space-5)',
          borderBottom: '1px solid var(--border-subtle)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          {getStatusBadge()}
          <span style={{ fontSize: '11px', color: 'var(--text-muted)', fontFamily: 'var(--font-mono)' }}>
            {task.id}
          </span>
        </div>
        <Button
          variant="ghost"
          size="sm"
          onClick={onClose}
          style={{ width: '28px', height: '28px', padding: 0 }}
          aria-label="Close details"
        >
          <X size={16} />
        </Button>
      </div>

      {/* Body Content */}
      <div
        style={{
          padding: 'var(--space-5)',
          overflowY: 'auto',
          flex: 1,
          display: 'flex',
          flexDirection: 'column',
          gap: 'var(--space-4)',
        }}
      >
        {/* Title */}
        <div>
          <h2 style={{ fontSize: '15px', fontWeight: 600, color: 'var(--text-primary)', lineHeight: 1.4 }}>
            {task.title}
          </h2>
          <div style={{ display: 'flex', alignItems: 'center', gap: '12px', marginTop: '6px' }}>
            <span style={{ fontSize: '12px', color: 'var(--text-muted)' }}>Created {task.createdAt}</span>
            {task.duration && (
              <>
                <span style={{ color: 'var(--border-subtle)' }}>•</span>
                <span style={{ fontSize: '12px', color: 'var(--text-muted)' }}>Duration: {task.duration}</span>
              </>
            )}
            <span style={{ color: 'var(--border-subtle)' }}>•</span>
            <span style={{ fontSize: '12px', color: 'var(--accent-primary)', fontWeight: 500 }}>
              {task.model}
            </span>
          </div>
        </div>

        {/* Original Prompt */}
        <div
          style={{
            backgroundColor: 'var(--bg-main)',
            border: '1px solid var(--border-subtle)',
            borderRadius: 'var(--radius-md)',
            padding: 'var(--space-3)',
          }}
        >
          <div
            style={{
              fontSize: '11px',
              fontWeight: 600,
              textTransform: 'uppercase',
              color: 'var(--text-muted)',
              marginBottom: '6px',
            }}
          >
            Prompt
          </div>
          <p style={{ fontSize: '13px', color: 'var(--text-primary)', whiteSpace: 'pre-wrap', lineHeight: 1.45 }}>
            {task.prompt}
          </p>
        </div>

        {/* Task Result Output */}
        {task.output && (
          <div
            style={{
              backgroundColor: 'var(--bg-main)',
              border: '1px solid var(--border-subtle)',
              borderRadius: 'var(--radius-md)',
              padding: 'var(--space-3)',
            }}
          >
            <div
              style={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'space-between',
                marginBottom: '8px',
              }}
            >
              <span
                style={{
                  fontSize: '11px',
                  fontWeight: 600,
                  textTransform: 'uppercase',
                  color: 'var(--text-muted)',
                }}
              >
                Task Result
              </span>
              <button
                onClick={handleCopyOutput}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: '4px',
                  fontSize: '11px',
                  color: 'var(--text-secondary)',
                  padding: '2px 6px',
                  borderRadius: 'var(--radius-sm)',
                }}
              >
                {copied ? <Check size={12} color="var(--status-success)" /> : <Copy size={12} />}
                <span>{copied ? 'Copied' : 'Copy'}</span>
              </button>
            </div>
            <pre
              style={{
                fontSize: '12px',
                color: 'var(--text-primary)',
                whiteSpace: 'pre-wrap',
                fontFamily: 'var(--font-mono)',
                lineHeight: 1.5,
              }}
            >
              {task.output}
            </pre>
          </div>
        )}

        {/* Step Logs */}
        <div>
          <div
            style={{
              fontSize: '11px',
              fontWeight: 600,
              textTransform: 'uppercase',
              color: 'var(--text-muted)',
              marginBottom: '8px',
            }}
          >
            Execution Logs ({task.logs.length})
          </div>
          <div
            style={{
              display: 'flex',
              flexDirection: 'column',
              gap: '6px',
              backgroundColor: 'var(--bg-main)',
              border: '1px solid var(--border-subtle)',
              borderRadius: 'var(--radius-md)',
              padding: 'var(--space-3)',
              fontFamily: 'var(--font-mono)',
              fontSize: '11px',
            }}
          >
            {task.logs.map((log, idx) => (
              <div key={idx} style={{ display: 'flex', alignItems: 'flex-start', gap: '8px' }}>
                <span style={{ color: 'var(--text-muted)', flexShrink: 0 }}>{log.timestamp}</span>
                <span
                  style={{
                    color:
                      log.level === 'action'
                        ? 'var(--accent-primary)'
                        : log.level === 'warn'
                        ? 'var(--status-warning)'
                        : log.level === 'error'
                        ? 'var(--status-danger)'
                        : 'var(--text-secondary)',
                  }}
                >
                  [{log.level.toUpperCase()}]
                </span>
                <span style={{ color: 'var(--text-primary)', wordBreak: 'break-word' }}>
                  {log.message}
                </span>
              </div>
            ))}
          </div>
        </div>

        {/* Token Usage Metrics */}
        {task.tokenUsage && (
          <div
            style={{
              display: 'grid',
              gridTemplateColumns: 'repeat(3, 1fr)',
              gap: '8px',
            }}
          >
            <div
              style={{
                padding: '8px',
                backgroundColor: 'var(--bg-main)',
                border: '1px solid var(--border-subtle)',
                borderRadius: 'var(--radius-md)',
                textAlign: 'center',
              }}
            >
              <div style={{ fontSize: '10px', color: 'var(--text-muted)' }}>PROMPT TOKENS</div>
              <div style={{ fontSize: '14px', fontWeight: 600, color: 'var(--text-primary)', marginTop: '2px' }}>
                {task.tokenUsage.promptTokens.toLocaleString()}
              </div>
            </div>
            <div
              style={{
                padding: '8px',
                backgroundColor: 'var(--bg-main)',
                border: '1px solid var(--border-subtle)',
                borderRadius: 'var(--radius-md)',
                textAlign: 'center',
              }}
            >
              <div style={{ fontSize: '10px', color: 'var(--text-muted)' }}>COMPLETION</div>
              <div style={{ fontSize: '14px', fontWeight: 600, color: 'var(--text-primary)', marginTop: '2px' }}>
                {task.tokenUsage.completionTokens.toLocaleString()}
              </div>
            </div>
            <div
              style={{
                padding: '8px',
                backgroundColor: 'var(--bg-main)',
                border: '1px solid var(--border-subtle)',
                borderRadius: 'var(--radius-md)',
                textAlign: 'center',
              }}
            >
              <div style={{ fontSize: '10px', color: 'var(--text-muted)' }}>TOTAL TOKENS</div>
              <div style={{ fontSize: '14px', fontWeight: 600, color: 'var(--accent-primary)', marginTop: '2px' }}>
                {task.tokenUsage.totalTokens.toLocaleString()}
              </div>
            </div>
          </div>
        )}

        {/* Artifacts */}
        {task.artifacts && task.artifacts.length > 0 && (
          <div>
            <div
              style={{
                fontSize: '11px',
                fontWeight: 600,
                textTransform: 'uppercase',
                color: 'var(--text-muted)',
                marginBottom: '8px',
              }}
            >
              Generated Artifacts ({task.artifacts.length})
            </div>
            <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
              {task.artifacts.map((a, i) => (
                <div
                  key={i}
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'space-between',
                    padding: '8px 10px',
                    backgroundColor: 'var(--bg-main)',
                    border: '1px solid var(--border-subtle)',
                    borderRadius: 'var(--radius-md)',
                  }}
                >
                  <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                    <FileCode size={14} color="var(--accent-primary)" />
                    <span style={{ fontSize: '12px', fontWeight: 500, color: 'var(--text-primary)' }}>
                      {a.name}
                    </span>
                    <span style={{ fontSize: '11px', color: 'var(--text-muted)' }}>({a.path})</span>
                  </div>
                  {a.sizeBytes && (
                    <span style={{ fontSize: '11px', color: 'var(--text-muted)', fontFamily: 'var(--font-mono)' }}>
                      {(a.sizeBytes / 1024).toFixed(1)} KB
                    </span>
                  )}
                </div>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
