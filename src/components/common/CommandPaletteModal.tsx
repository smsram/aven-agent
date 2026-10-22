import React, { useState, useEffect, useRef } from 'react';
import { Search, SquarePen, Activity, Blocks, Settings, Shield, ArrowRight } from 'lucide-react';
import { NavigationTab, AgentTask } from '../../types';

export interface CommandPaletteModalProps {
  isOpen: boolean;
  onClose: () => void;
  onNavigateTab: (tab: NavigationTab) => void;
  onOpenSettings: () => void;
  tasks: AgentTask[];
  onSelectTask: (task: AgentTask) => void;
  onSubmitQuickPrompt: (prompt: string) => void;
}

export const CommandPaletteModal: React.FC<CommandPaletteModalProps> = ({
  isOpen,
  onClose,
  onNavigateTab,
  onOpenSettings,
  tasks,
  onSelectTask,
  onSubmitQuickPrompt,
}) => {
  const [query, setQuery] = useState('');
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (isOpen) {
      setQuery('');
      setTimeout(() => inputRef.current?.focus(), 50);
    }
  }, [isOpen]);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
        e.preventDefault();
        if (isOpen) onClose();
      }
      if (e.key === 'Escape' && isOpen) {
        onClose();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  const commands = [
    {
      id: 'new-chat',
      title: 'New Chat / Task',
      category: 'Navigation',
      icon: <SquarePen size={15} color="var(--accent-primary)" />,
      shortcut: 'Ctrl+1',
      action: () => {
        onNavigateTab('new-chat');
        onClose();
      },
    },
    {
      id: 'activity',
      title: 'Activity History',
      category: 'Navigation',
      icon: <Activity size={15} color="var(--accent-primary)" />,
      shortcut: 'Ctrl+2',
      action: () => {
        onNavigateTab('activity');
        onClose();
      },
    },
    {
      id: 'plugins',
      title: 'Plugins & MCP',
      category: 'Navigation',
      icon: <Blocks size={15} color="var(--accent-primary)" />,
      shortcut: 'Ctrl+3',
      action: () => {
        onNavigateTab('plugins');
        onClose();
      },
    },
    {
      id: 'settings',
      title: 'Application Settings',
      category: 'Preferences',
      icon: <Settings size={15} color="var(--accent-primary)" />,
      shortcut: 'Ctrl+,',
      action: () => {
        onOpenSettings();
        onClose();
      },
    },
    {
      id: 'quick-audit',
      title: 'Run Workspace Security & Dependency Audit',
      category: 'Quick Action',
      icon: <Shield size={15} color="var(--status-success)" />,
      action: () => {
        onSubmitQuickPrompt('Run dependency and security audit across workspace');
        onClose();
      },
    },
  ];

  const filteredCommands = commands.filter((c) =>
    c.title.toLowerCase().includes(query.toLowerCase()) ||
    c.category.toLowerCase().includes(query.toLowerCase())
  );

  const filteredTasks = tasks.filter((t) =>
    t.title.toLowerCase().includes(query.toLowerCase()) ||
    t.prompt.toLowerCase().includes(query.toLowerCase())
  ).slice(0, 4);

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        backgroundColor: 'rgba(10, 11, 14, 0.72)',
        display: 'flex',
        alignItems: 'flex-start',
        justifyContent: 'center',
        paddingTop: '12vh',
        zIndex: 1000,
      }}
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        style={{
          width: '100%',
          maxWidth: '540px',
          backgroundColor: 'var(--bg-panel)',
          border: '1px solid var(--border-subtle)',
          borderRadius: 'var(--radius-xl)',
          overflow: 'hidden',
          boxShadow: '0 16px 48px rgba(0,0,0,0.6)',
          display: 'flex',
          flexDirection: 'column',
        }}
      >
        {/* Search Input Bar */}
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            padding: '12px 16px',
            borderBottom: '1px solid var(--border-subtle)',
            gap: '10px',
          }}
        >
          <Search size={15} color="var(--text-muted)" />
          <input
            ref={inputRef}
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Type a command or search tasks..."
            style={{
              flex: 1,
              background: 'transparent',
              border: 'none',
              color: 'var(--text-primary)',
              fontSize: '13px',
              outline: 'none',
            }}
          />
          <kbd
            style={{
              fontSize: '10px',
              padding: '2px 5px',
              backgroundColor: 'var(--bg-input)',
              border: '1px solid var(--border-subtle)',
              borderRadius: 'var(--radius-sm)',
              color: 'var(--text-muted)',
              fontFamily: 'var(--font-mono)',
            }}
          >
            ESC
          </kbd>
        </div>

        {/* Results List */}
        <div style={{ maxHeight: '360px', overflowY: 'auto', padding: '6px' }}>
          {filteredCommands.length > 0 && (
            <div style={{ marginBottom: '6px' }}>
              <div
                style={{
                  fontSize: '10px',
                  fontWeight: 600,
                  textTransform: 'uppercase',
                  color: 'var(--text-muted)',
                  padding: '6px 10px 4px 10px',
                  letterSpacing: '0.04em',
                }}
              >
                Commands
              </div>
              {filteredCommands.map((cmd) => (
                <div
                  key={cmd.id}
                  onClick={cmd.action}
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'space-between',
                    padding: '8px 10px',
                    borderRadius: 'var(--radius-md)',
                    cursor: 'pointer',
                    transition: 'background-color var(--transition-fast)',
                  }}
                  onMouseEnter={(e) => (e.currentTarget.style.backgroundColor = 'var(--bg-panel-hover)')}
                  onMouseLeave={(e) => (e.currentTarget.style.backgroundColor = 'transparent')}
                >
                  <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
                    <span style={{ display: 'flex' }}>{cmd.icon}</span>
                    <span style={{ fontSize: '13px', color: 'var(--text-primary)', fontWeight: 500 }}>
                      {cmd.title}
                    </span>
                  </div>
                  {cmd.shortcut && (
                    <kbd
                      style={{
                        fontSize: '10px',
                        padding: '1px 5px',
                        backgroundColor: 'var(--bg-input)',
                        border: '1px solid var(--border-subtle)',
                        borderRadius: 'var(--radius-sm)',
                        color: 'var(--text-muted)',
                        fontFamily: 'var(--font-mono)',
                      }}
                    >
                      {cmd.shortcut}
                    </kbd>
                  )}
                </div>
              ))}
            </div>
          )}

          {filteredTasks.length > 0 && (
            <div>
              <div
                style={{
                  fontSize: '10px',
                  fontWeight: 600,
                  textTransform: 'uppercase',
                  color: 'var(--text-muted)',
                  padding: '6px 10px 4px 10px',
                  letterSpacing: '0.04em',
                }}
              >
                Recent Tasks
              </div>
              {filteredTasks.map((t) => (
                <div
                  key={t.id}
                  onClick={() => {
                    onSelectTask(t);
                    onClose();
                  }}
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'space-between',
                    padding: '8px 10px',
                    borderRadius: 'var(--radius-md)',
                    cursor: 'pointer',
                  }}
                  onMouseEnter={(e) => (e.currentTarget.style.backgroundColor = 'var(--bg-panel-hover)')}
                  onMouseLeave={(e) => (e.currentTarget.style.backgroundColor = 'transparent')}
                >
                  <span className="truncate" style={{ fontSize: '12px', color: 'var(--text-secondary)' }}>
                    {t.title}
                  </span>
                  <ArrowRight size={12} color="var(--text-muted)" />
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
