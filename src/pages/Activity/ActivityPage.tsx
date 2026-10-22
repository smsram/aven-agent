import React, { useState, useMemo } from 'react';
import { Search, ChevronDown, Trash2, ArrowUpRight } from 'lucide-react';
import { AgentTask, TaskStatus } from '../../types';

export interface ActivityPageProps {
  tasks: AgentTask[];
  onClearTasks: () => void;
  onNavigateNewChat: () => void;
  onSelectTask?: (task: AgentTask) => void;
}

export const ActivityPage: React.FC<ActivityPageProps> = ({
  tasks,
  onClearTasks,
  onNavigateNewChat,
  onSelectTask,
}) => {
  const [statusFilter, setStatusFilter] = useState<'all' | TaskStatus>('all');
  const [searchQuery, setSearchQuery] = useState('');

  const filteredTasks = useMemo(() => {
    return tasks.filter((task) => {
      const matchesStatus =
        statusFilter === 'all'
          ? true
          : statusFilter === 'running'
          ? task.status === 'running' || task.status === 'queued'
          : task.status === statusFilter;

      const matchesSearch =
        searchQuery === '' ||
        task.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
        task.prompt.toLowerCase().includes(searchQuery.toLowerCase()) ||
        (task.output && task.output.toLowerCase().includes(searchQuery.toLowerCase())) ||
        task.id.toLowerCase().includes(searchQuery.toLowerCase());

      return matchesStatus && matchesSearch;
    });
  }, [tasks, statusFilter, searchQuery]);

  const renderStatus = (status: TaskStatus) => {
    switch (status) {
      case 'running':
      case 'queued':
        return (
          <span style={{ display: 'inline-flex', alignItems: 'center', gap: '6px', fontSize: '12px', color: 'var(--text-accent)' }}>
            <span style={{ width: '6px', height: '6px', borderRadius: '50%', backgroundColor: 'var(--accent-primary)', flexShrink: 0 }} />
            <span>In Progress</span>
          </span>
        );
      case 'completed':
        return (
          <span style={{ display: 'inline-flex', alignItems: 'center', gap: '6px', fontSize: '12px', color: 'var(--status-success)' }}>
            <span style={{ width: '6px', height: '6px', borderRadius: '50%', backgroundColor: 'var(--status-success)', flexShrink: 0 }} />
            <span>Completed</span>
          </span>
        );
      case 'failed':
        return (
          <span style={{ display: 'inline-flex', alignItems: 'center', gap: '6px', fontSize: '12px', color: 'var(--status-danger)' }}>
            <span style={{ width: '6px', height: '6px', borderRadius: '50%', backgroundColor: 'var(--status-danger)', flexShrink: 0 }} />
            <span>Failed</span>
          </span>
        );
      case 'paused':
        return (
          <span style={{ display: 'inline-flex', alignItems: 'center', gap: '6px', fontSize: '12px', color: 'var(--status-warning)' }}>
            <span style={{ width: '6px', height: '6px', borderRadius: '50%', backgroundColor: 'var(--status-warning)', flexShrink: 0 }} />
            <span>Paused</span>
          </span>
        );
      default:
        return (
          <span style={{ display: 'inline-flex', alignItems: 'center', gap: '6px', fontSize: '12px', color: 'var(--text-muted)' }}>
            <span style={{ width: '6px', height: '6px', borderRadius: '50%', backgroundColor: 'var(--text-muted)', flexShrink: 0 }} />
            <span>{status}</span>
          </span>
        );
    }
  };

  return (
    <div
      style={{
        height: '100%',
        display: 'flex',
        flexDirection: 'column',
        overflow: 'hidden',
        position: 'relative',
        backgroundColor: 'var(--bg-main)',
      }}
    >
      {/* Header Bar */}
      <div
        style={{
          padding: '12px 24px',
          borderBottom: '1px solid var(--border-subtle)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          flexShrink: 0,
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          <h1 style={{ fontSize: '14px', fontWeight: 600, color: 'var(--text-primary)' }}>
            Activity
          </h1>
          <span style={{ fontSize: '12px', color: 'var(--text-muted)', fontFamily: 'var(--font-mono)' }}>
            ({tasks.length})
          </span>
        </div>

        {/* Search and Compact Filter Dropdown */}
        <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
          {/* Search Input */}
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              backgroundColor: 'var(--bg-panel)',
              border: '1px solid var(--border-subtle)',
              borderRadius: 'var(--radius-md)',
              padding: '0 8px',
              height: '28px',
              width: '210px',
            }}
          >
            <Search size={13} color="var(--text-muted)" style={{ marginRight: '6px', flexShrink: 0 }} />
            <input
              type="text"
              placeholder="Search history..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              style={{
                width: '100%',
                background: 'transparent',
                border: 'none',
                color: 'var(--text-primary)',
                caretColor: 'var(--accent-primary)',
                cursor: 'text',
                fontSize: '12px',
                outline: 'none',
              }}
            />
          </div>

          {/* Compact Dropdown Filter */}
          <div style={{ position: 'relative', display: 'flex', alignItems: 'center' }}>
            <select
              value={statusFilter}
              onChange={(e) => setStatusFilter(e.target.value as any)}
              style={{
                height: '28px',
                padding: '0 24px 0 10px',
                backgroundColor: 'var(--bg-panel)',
                border: '1px solid var(--border-subtle)',
                borderRadius: 'var(--radius-md)',
                color: 'var(--text-secondary)',
                fontSize: '12px',
                cursor: 'pointer',
                outline: 'none',
                appearance: 'none',
              }}
            >
              <option value="all">All</option>
              <option value="running">In Progress</option>
              <option value="completed">Completed</option>
              <option value="failed">Failed</option>
              <option value="paused">Paused</option>
            </select>
            <ChevronDown
              size={12}
              color="var(--text-muted)"
              style={{ position: 'absolute', right: '8px', pointerEvents: 'none' }}
            />
          </div>

          {/* Clear button */}
          {tasks.length > 0 && (
            <button
              onClick={onClearTasks}
              style={{
                height: '28px',
                padding: '0 8px',
                borderRadius: 'var(--radius-md)',
                color: 'var(--text-muted)',
                backgroundColor: 'transparent',
                border: '1px solid transparent',
                fontSize: '12px',
                display: 'flex',
                alignItems: 'center',
                gap: '4px',
                cursor: 'pointer',
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.color = 'var(--status-danger)';
                e.currentTarget.style.backgroundColor = 'var(--bg-panel)';
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.color = 'var(--text-muted)';
                e.currentTarget.style.backgroundColor = 'transparent';
              }}
              title="Clear history"
            >
              <Trash2 size={13} />
            </button>
          )}
        </div>
      </div>

      {/* Clean Chronological List */}
      <div style={{ flex: 1, overflowY: 'auto' }}>
        {filteredTasks.length === 0 ? (
          <div
            style={{
              padding: '60px 24px',
              textAlign: 'center',
              color: 'var(--text-secondary)',
            }}
          >
            <p style={{ fontSize: '13px', fontWeight: 500, color: 'var(--text-primary)' }}>
              No activities found
            </p>
            <p style={{ fontSize: '12px', color: 'var(--text-muted)', marginTop: '4px' }}>
              {searchQuery || statusFilter !== 'all'
                ? 'Try adjusting your search query or filter criteria.'
                : 'Tasks and chat conversations will appear here chronologically.'}
            </p>
            {searchQuery || statusFilter !== 'all' ? (
              <button
                onClick={() => {
                  setSearchQuery('');
                  setStatusFilter('all');
                }}
                style={{
                  marginTop: '12px',
                  padding: '5px 12px',
                  borderRadius: 'var(--radius-md)',
                  backgroundColor: 'var(--bg-panel)',
                  border: '1px solid var(--border-subtle)',
                  color: 'var(--text-primary)',
                  fontSize: '12px',
                  cursor: 'pointer',
                }}
              >
                Reset filters
              </button>
            ) : (
              <button
                onClick={onNavigateNewChat}
                style={{
                  marginTop: '12px',
                  padding: '5px 12px',
                  borderRadius: 'var(--radius-md)',
                  backgroundColor: 'var(--accent-primary)',
                  color: '#FFFFFF',
                  fontSize: '12px',
                  fontWeight: 500,
                  cursor: 'pointer',
                  border: 'none',
                }}
              >
                Start a task
              </button>
            )}
          </div>
        ) : (
          <div style={{ display: 'flex', flexDirection: 'column' }}>
            {filteredTasks.map((task) => (
              <div
                key={task.id}
                onClick={() => onSelectTask?.(task)}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'space-between',
                  padding: '10px 24px',
                  borderBottom: '1px solid var(--border-subtle)',
                  cursor: 'pointer',
                  transition: 'background-color var(--transition-fast)',
                }}
                onMouseEnter={(e) => (e.currentTarget.style.backgroundColor = 'var(--bg-panel-hover)')}
                onMouseLeave={(e) => (e.currentTarget.style.backgroundColor = 'transparent')}
                title="Open conversation"
              >
                {/* Left: Status & Title */}
                <div style={{ display: 'flex', alignItems: 'center', gap: '16px', minWidth: 0, flex: 1, paddingRight: '16px' }}>
                  <div style={{ width: '100px', flexShrink: 0 }}>
                    {renderStatus(task.status)}
                  </div>
                  <div style={{ minWidth: 0, flex: 1 }}>
                    <div
                      className="truncate"
                      style={{ fontSize: '13px', fontWeight: 500, color: 'var(--text-primary)' }}
                    >
                      {task.title}
                    </div>
                  </div>
                </div>

                {/* Right: Duration, Model & Timestamp metadata */}
                <div style={{ display: 'flex', alignItems: 'center', gap: '16px', flexShrink: 0 }}>
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
                  <span
                    style={{
                      fontSize: '11px',
                      color: 'var(--text-secondary)',
                      padding: '1px 6px',
                      borderRadius: 'var(--radius-sm)',
                      backgroundColor: 'var(--bg-panel)',
                      border: '1px solid var(--border-subtle)',
                    }}
                  >
                    {task.model}
                  </span>
                  <span style={{ fontSize: '12px', color: 'var(--text-muted)', minWidth: '70px', textAlign: 'right' }}>
                    {task.createdAt}
                  </span>
                  <ArrowUpRight size={13} color="var(--text-muted)" />
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
};
