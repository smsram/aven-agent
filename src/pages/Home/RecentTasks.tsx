import React from 'react';
import { CheckCircle2, Clock, ArrowRight, AlertCircle } from 'lucide-react';
import { AgentTask } from '../../types';
import { Badge } from '../../components/common/Badge';

export interface RecentTasksProps {
  tasks: AgentTask[];
  onSelectTask: (task: AgentTask) => void;
  onViewAllActivity: () => void;
}

export const RecentTasks: React.FC<RecentTasksProps> = ({
  tasks,
  onSelectTask,
  onViewAllActivity,
}) => {
  const getStatusIcon = (status: AgentTask['status']) => {
    switch (status) {
      case 'completed':
        return <CheckCircle2 size={13} color="var(--status-success)" />;
      case 'running':
        return <span style={{ width: '8px', height: '8px', borderRadius: '50%', backgroundColor: 'var(--accent-primary)', display: 'inline-block' }} />;
      case 'failed':
        return <AlertCircle size={13} color="var(--status-danger)" />;
      default:
        return <Clock size={13} color="var(--text-muted)" />;
    }
  };

  return (
    <div style={{ width: '100%', marginTop: 'var(--space-6)' }}>
      {/* Header */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          marginBottom: 'var(--space-3)',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          <span
            style={{
              fontSize: '12px',
              fontWeight: 600,
              textTransform: 'uppercase',
              letterSpacing: '0.04em',
              color: 'var(--text-muted)',
            }}
          >
            Recent Tasks
          </span>
          <span
            style={{
              fontSize: '11px',
              padding: '1px 6px',
              borderRadius: 'var(--radius-full)',
              backgroundColor: 'var(--bg-panel)',
              color: 'var(--text-secondary)',
              border: '1px solid var(--border-subtle)',
            }}
          >
            {tasks.length}
          </span>
        </div>

        <button
          onClick={onViewAllActivity}
          style={{
            fontSize: '12px',
            color: 'var(--accent-primary)',
            display: 'flex',
            alignItems: 'center',
            gap: '4px',
            padding: '2px 6px',
            borderRadius: 'var(--radius-sm)',
          }}
          onMouseEnter={(e) => (e.currentTarget.style.color = 'var(--accent-primary-hover)')}
          onMouseLeave={(e) => (e.currentTarget.style.color = 'var(--accent-primary)')}
        >
          <span>View all activity</span>
          <ArrowRight size={12} />
        </button>
      </div>

      {/* Task List */}
      {tasks.length === 0 ? (
        <div
          style={{
            padding: 'var(--space-6)',
            textAlign: 'center',
            backgroundColor: 'var(--bg-panel)',
            border: '1px solid var(--border-subtle)',
            borderRadius: 'var(--radius-lg)',
            color: 'var(--text-secondary)',
          }}
        >
          <p style={{ fontSize: '13px' }}>No tasks executed yet.</p>
          <p style={{ fontSize: '12px', color: 'var(--text-muted)', marginTop: '4px' }}>
            Enter a prompt above to run your first agent task.
          </p>
        </div>
      ) : (
        <div
          style={{
            display: 'flex',
            flexDirection: 'column',
            backgroundColor: 'var(--bg-panel)',
            border: '1px solid var(--border-subtle)',
            borderRadius: 'var(--radius-lg)',
            overflow: 'hidden',
          }}
        >
          {tasks.map((task, idx) => (
            <div
              key={task.id}
              onClick={() => onSelectTask(task)}
              style={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'space-between',
                padding: '10px 14px',
                borderBottom: idx === tasks.length - 1 ? 'none' : '1px solid var(--border-subtle)',
                cursor: 'pointer',
                transition: 'background-color var(--transition-fast)',
              }}
              onMouseEnter={(e) => (e.currentTarget.style.backgroundColor = 'var(--bg-panel-hover)')}
              onMouseLeave={(e) => (e.currentTarget.style.backgroundColor = 'transparent')}
            >
              {/* Left: Status icon & Title */}
              <div style={{ display: 'flex', alignItems: 'center', gap: '10px', minWidth: 0, flex: 1 }}>
                <span style={{ display: 'flex', alignItems: 'center', flexShrink: 0 }}>
                  {getStatusIcon(task.status)}
                </span>
                <span
                  className="truncate"
                  style={{
                    fontSize: '13px',
                    fontWeight: 500,
                    color: 'var(--text-primary)',
                    letterSpacing: '-0.01em',
                  }}
                >
                  {task.title}
                </span>
              </div>

              {/* Right: Model badge, duration & timestamp */}
              <div
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: '12px',
                  flexShrink: 0,
                  marginLeft: 'var(--space-4)',
                }}
              >
                <Badge variant="default" size="sm">
                  {task.model}
                </Badge>
                {task.duration && (
                  <span
                    style={{
                      fontSize: '11px',
                      color: 'var(--text-muted)',
                      fontFamily: 'var(--font-mono)',
                    }}
                  >
                    {task.duration}
                  </span>
                )}
                <span style={{ fontSize: '12px', color: 'var(--text-muted)', minWidth: '70px', textAlign: 'right' }}>
                  {task.createdAt}
                </span>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};
