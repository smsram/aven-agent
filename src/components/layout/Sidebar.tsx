import React from 'react';
import { SquarePen, Activity, Blocks, Settings, PanelLeftClose, PanelLeftOpen } from 'lucide-react';
import { NavigationTab } from '../../types';

export interface SidebarProps {
  activeTab: NavigationTab;
  onTabChange: (tab: NavigationTab) => void;
  onNewChat: () => void;
  onOpenSettings: () => void;
  isSettingsOpen: boolean;
  isCollapsed: boolean;
  onToggleCollapse: () => void;
  taskCount: number;
}

export const Sidebar: React.FC<SidebarProps> = ({
  activeTab,
  onTabChange,
  onNewChat,
  onOpenSettings,
  isSettingsOpen,
  isCollapsed,
  onToggleCollapse,
  taskCount,
}) => {
  return (
    <aside
      style={{
        width: isCollapsed ? '52px' : '210px',
        height: '100%',
        backgroundColor: 'var(--bg-sidebar)',
        borderRight: '1px solid var(--border-subtle)',
        display: 'flex',
        flexDirection: 'column',
        justifyContent: 'space-between',
        flexShrink: 0,
        transition: 'width var(--transition-normal)',
        overflow: 'hidden',
        zIndex: 20,
        userSelect: 'none',
      }}
    >
      {/* Primary Navigation: New Chat, Activity, Plugins (without duplicate header logo/name) */}
      <nav
        style={{
          padding: isCollapsed ? '12px 6px' : '12px 8px',
          display: 'flex',
          flexDirection: 'column',
          gap: '3px',
        }}
      >
        {/* 1. New Chat: Visually neutral without persistent selected highlight */}
        <button
          type="button"
          onClick={onNewChat}
          style={{
            width: '100%',
            height: '32px',
            padding: isCollapsed ? '0' : '0 10px',
            justifyContent: isCollapsed ? 'center' : 'flex-start',
            borderRadius: 'var(--radius-md)',
            backgroundColor: 'transparent',
            color: 'var(--text-secondary)',
            border: '1px solid transparent',
            cursor: 'pointer',
            transition: 'background-color var(--transition-fast), color var(--transition-fast)',
            display: 'flex',
            alignItems: 'center',
            gap: '10px',
          }}
          onMouseEnter={(e) => {
            e.currentTarget.style.backgroundColor = 'var(--bg-sidebar-hover)';
            e.currentTarget.style.color = 'var(--text-primary)';
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.backgroundColor = 'transparent';
            e.currentTarget.style.color = 'var(--text-secondary)';
          }}
          title={isCollapsed ? 'New Chat' : undefined}
        >
          <SquarePen size={16} strokeWidth={1.75} style={{ flexShrink: 0 }} />
          {!isCollapsed && (
            <span style={{ fontSize: '13px', fontWeight: 500 }}>
              New Chat
            </span>
          )}
        </button>

        {/* 2. Activity */}
        {(() => {
          const isActive = activeTab === 'activity' && !isSettingsOpen;
          return (
            <button
              type="button"
              onClick={() => onTabChange('activity')}
              style={{
                width: '100%',
                height: '32px',
                padding: isCollapsed ? '0' : '0 10px',
                justifyContent: isCollapsed ? 'center' : 'space-between',
                borderRadius: 'var(--radius-md)',
                backgroundColor: isActive ? 'var(--bg-sidebar-active)' : 'transparent',
                color: isActive ? 'var(--text-primary)' : 'var(--text-secondary)',
                border: isActive ? '1px solid var(--accent-border)' : '1px solid transparent',
                cursor: 'pointer',
                transition: 'background-color var(--transition-fast), color var(--transition-fast)',
                display: 'flex',
                alignItems: 'center',
                position: 'relative',
              }}
              onMouseEnter={(e) => {
                if (!isActive) {
                  e.currentTarget.style.backgroundColor = 'var(--bg-sidebar-hover)';
                  e.currentTarget.style.color = 'var(--text-primary)';
                }
              }}
              onMouseLeave={(e) => {
                if (!isActive) {
                  e.currentTarget.style.backgroundColor = 'transparent';
                  e.currentTarget.style.color = 'var(--text-secondary)';
                }
              }}
              title={isCollapsed ? 'Activity' : undefined}
            >
              <div
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: '10px',
                  color: isActive ? 'var(--accent-primary)' : 'inherit',
                }}
              >
                <Activity size={16} strokeWidth={1.75} style={{ flexShrink: 0 }} />
                {!isCollapsed && (
                  <span style={{ fontSize: '13px', fontWeight: isActive ? 500 : 400 }}>
                    Activity
                  </span>
                )}
              </div>

              {!isCollapsed && taskCount > 0 && (
                <span
                  style={{
                    fontSize: '11px',
                    padding: '1px 6px',
                    borderRadius: 'var(--radius-full)',
                    backgroundColor: isActive ? 'var(--accent-subtle)' : 'var(--bg-panel)',
                    color: isActive ? 'var(--accent-primary)' : 'var(--text-muted)',
                    border: '1px solid var(--border-subtle)',
                    fontFamily: 'var(--font-mono)',
                  }}
                >
                  {taskCount}
                </span>
              )}

              {isActive && (
                <span
                  style={{
                    position: 'absolute',
                    left: '-2px',
                    top: '6px',
                    bottom: '6px',
                    width: '3px',
                    borderRadius: '0 2px 2px 0',
                    backgroundColor: 'var(--accent-primary)',
                  }}
                />
              )}
            </button>
          );
        })()}

        {/* 3. Plugins */}
        {(() => {
          const isActive = activeTab === 'plugins' && !isSettingsOpen;
          return (
            <button
              type="button"
              onClick={() => onTabChange('plugins')}
              style={{
                width: '100%',
                height: '32px',
                padding: isCollapsed ? '0' : '0 10px',
                justifyContent: isCollapsed ? 'center' : 'flex-start',
                borderRadius: 'var(--radius-md)',
                backgroundColor: isActive ? 'var(--bg-sidebar-active)' : 'transparent',
                color: isActive ? 'var(--text-primary)' : 'var(--text-secondary)',
                border: isActive ? '1px solid var(--accent-border)' : '1px solid transparent',
                cursor: 'pointer',
                transition: 'background-color var(--transition-fast), color var(--transition-fast)',
                display: 'flex',
                alignItems: 'center',
                gap: '10px',
                position: 'relative',
              }}
              onMouseEnter={(e) => {
                if (!isActive) {
                  e.currentTarget.style.backgroundColor = 'var(--bg-sidebar-hover)';
                  e.currentTarget.style.color = 'var(--text-primary)';
                }
              }}
              onMouseLeave={(e) => {
                if (!isActive) {
                  e.currentTarget.style.backgroundColor = 'transparent';
                  e.currentTarget.style.color = 'var(--text-secondary)';
                }
              }}
              title={isCollapsed ? 'Plugins' : undefined}
            >
              <Blocks
                size={16}
                strokeWidth={1.75}
                color={isActive ? 'var(--accent-primary)' : 'currentColor'}
                style={{ flexShrink: 0 }}
              />
              {!isCollapsed && (
                <span style={{ fontSize: '13px', fontWeight: isActive ? 500 : 400 }}>
                  Plugins
                </span>
              )}

              {isActive && (
                <span
                  style={{
                    position: 'absolute',
                    left: '-2px',
                    top: '6px',
                    bottom: '6px',
                    width: '3px',
                    borderRadius: '0 2px 2px 0',
                    backgroundColor: 'var(--accent-primary)',
                  }}
                />
              )}
            </button>
          );
        })()}
      </nav>

      {/* Bottom Section: Collapse/Expand directly above Settings, and Settings */}
      <div
        style={{
          padding: isCollapsed ? '8px 6px' : '8px 8px',
          borderTop: '1px solid var(--border-subtle)',
          display: 'flex',
          flexDirection: 'column',
          gap: '3px',
        }}
      >
        {/* Collapse / Expand Toggle Button directly above Settings */}
        <button
          type="button"
          onClick={onToggleCollapse}
          style={{
            width: '100%',
            height: '32px',
            padding: isCollapsed ? '0' : '0 10px',
            justifyContent: isCollapsed ? 'center' : 'flex-start',
            borderRadius: 'var(--radius-md)',
            backgroundColor: 'transparent',
            color: 'var(--text-secondary)',
            border: '1px solid transparent',
            cursor: 'pointer',
            transition: 'background-color var(--transition-fast), color var(--transition-fast)',
            display: 'flex',
            alignItems: 'center',
            gap: '10px',
          }}
          onMouseEnter={(e) => {
            e.currentTarget.style.backgroundColor = 'var(--bg-sidebar-hover)';
            e.currentTarget.style.color = 'var(--text-primary)';
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.backgroundColor = 'transparent';
            e.currentTarget.style.color = 'var(--text-secondary)';
          }}
          title={isCollapsed ? 'Expand sidebar (Ctrl+B)' : 'Collapse sidebar (Ctrl+B)'}
        >
          {isCollapsed ? (
            <PanelLeftOpen size={16} strokeWidth={1.75} style={{ flexShrink: 0 }} />
          ) : (
            <PanelLeftClose size={16} strokeWidth={1.75} style={{ flexShrink: 0 }} />
          )}
          {!isCollapsed && (
            <span style={{ fontSize: '13px', fontWeight: 400 }}>
              Collapse
            </span>
          )}
        </button>

        {/* Settings Button (never persistently highlighted) */}
        <button
          type="button"
          onClick={onOpenSettings}
          style={{
            width: '100%',
            height: '32px',
            padding: isCollapsed ? '0' : '0 10px',
            justifyContent: isCollapsed ? 'center' : 'flex-start',
            borderRadius: 'var(--radius-md)',
            backgroundColor: 'transparent',
            color: 'var(--text-secondary)',
            border: '1px solid transparent',
            cursor: 'pointer',
            transition: 'background-color var(--transition-fast), color var(--transition-fast)',
            display: 'flex',
            alignItems: 'center',
            gap: '10px',
          }}
          onMouseEnter={(e) => {
            e.currentTarget.style.backgroundColor = 'var(--bg-sidebar-hover)';
            e.currentTarget.style.color = 'var(--text-primary)';
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.backgroundColor = 'transparent';
            e.currentTarget.style.color = 'var(--text-secondary)';
          }}
          title={isCollapsed ? 'Settings (Ctrl+,)' : undefined}
        >
          <Settings size={16} strokeWidth={1.75} style={{ flexShrink: 0 }} />
          {!isCollapsed && (
            <span style={{ fontSize: '13px', fontWeight: 400 }}>
              Settings
            </span>
          )}
        </button>
      </div>
    </aside>
  );
};
