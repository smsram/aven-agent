import React, { useState } from 'react';
import { Blocks, Plus, Trash2, Terminal, Globe, Power } from 'lucide-react';
import { McpPlugin } from '../../types';
import { Button } from '../../components/common/Button';
import { AddMcpModal } from './AddMcpModal';

export interface PluginsPageProps {
  plugins: McpPlugin[];
  onAddPlugin: (plugin: Omit<McpPlugin, 'id' | 'createdAt' | 'status'>) => void;
  onTogglePlugin: (id: string) => void;
  onDeletePlugin: (id: string) => void;
}

export const PluginsPage: React.FC<PluginsPageProps> = ({
  plugins,
  onAddPlugin,
  onTogglePlugin,
  onDeletePlugin,
}) => {
  const [isAddModalOpen, setIsAddModalOpen] = useState(false);

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
        <div>
          <h1 style={{ fontSize: '14px', fontWeight: 600, color: 'var(--text-primary)' }}>
            Plugins & MCP Integrations
          </h1>
          <p style={{ fontSize: '12px', color: 'var(--text-secondary)', marginTop: '2px' }}>
            External Model Context Protocol (MCP) servers and tools
          </p>
        </div>

        <Button
          variant="primary"
          size="sm"
          icon={<Plus size={13} />}
          onClick={() => setIsAddModalOpen(true)}
        >
          Add MCP Server
        </Button>
      </div>

      {/* Main Content Area */}
      <div style={{ flex: 1, overflowY: 'auto', padding: '24px' }}>
        {plugins.length === 0 ? (
          /* Clear Empty State as strictly required */
          <div
            style={{
              maxWidth: '480px',
              margin: '80px auto',
              textAlign: 'center',
              display: 'flex',
              flexDirection: 'column',
              alignItems: 'center',
              padding: '32px 24px',
              backgroundColor: 'var(--bg-panel)',
              border: '1px solid var(--border-subtle)',
              borderRadius: 'var(--radius-lg)',
            }}
          >
            <div
              style={{
                width: '40px',
                height: '40px',
                borderRadius: '8px',
                backgroundColor: 'var(--bg-main)',
                border: '1px solid var(--border-subtle)',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                color: 'var(--text-muted)',
                marginBottom: '16px',
              }}
            >
              <Blocks size={20} strokeWidth={1.5} />
            </div>

            <h2 style={{ fontSize: '15px', fontWeight: 600, color: 'var(--text-primary)', marginBottom: '6px' }}>
              No external plugins configured
            </h2>

            <p style={{ fontSize: '12px', color: 'var(--text-secondary)', lineHeight: 1.5, marginBottom: '20px', maxWidth: '380px' }}>
              Connect local or remote Model Context Protocol (MCP) servers via stdio or SSE to extend Aven's autonomous tool capabilities.
            </p>

            <Button
              variant="primary"
              size="md"
              icon={<Plus size={14} />}
              onClick={() => setIsAddModalOpen(true)}
            >
              Add your first MCP Server
            </Button>
          </div>
        ) : (
          /* List of configured MCP Servers */
          <div style={{ maxWidth: '820px', margin: '0 auto', display: 'flex', flexDirection: 'column', gap: '10px' }}>
            {plugins.map((plugin) => {
              const isActive = plugin.status === 'active';
              return (
                <div
                  key={plugin.id}
                  style={{
                    backgroundColor: 'var(--bg-panel)',
                    border: '1px solid var(--border-subtle)',
                    borderRadius: 'var(--radius-md)',
                    padding: '12px 16px',
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'space-between',
                  }}
                >
                  <div style={{ display: 'flex', alignItems: 'flex-start', gap: '12px' }}>
                    <div
                      style={{
                        width: '30px',
                        height: '30px',
                        borderRadius: 'var(--radius-sm)',
                        backgroundColor: 'var(--bg-main)',
                        border: '1px solid var(--border-subtle)',
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        color: isActive ? 'var(--accent-primary)' : 'var(--text-muted)',
                        flexShrink: 0,
                      }}
                    >
                      {plugin.transport === 'stdio' ? <Terminal size={15} /> : <Globe size={15} />}
                    </div>

                    <div>
                      <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                        <span style={{ fontSize: '13px', fontWeight: 600, color: 'var(--text-primary)' }}>
                          {plugin.name}
                        </span>
                        <span
                          style={{
                            fontSize: '10px',
                            padding: '1px 5px',
                            borderRadius: '3px',
                            backgroundColor: 'var(--bg-main)',
                            border: '1px solid var(--border-subtle)',
                            color: 'var(--text-muted)',
                            fontFamily: 'var(--font-mono)',
                            textTransform: 'uppercase',
                          }}
                        >
                          {plugin.transport}
                        </span>
                        <span
                          style={{
                            fontSize: '11px',
                            display: 'inline-flex',
                            alignItems: 'center',
                            gap: '4px',
                            color: isActive ? 'var(--status-success)' : 'var(--text-muted)',
                          }}
                        >
                          <span
                            style={{
                              width: '5px',
                              height: '5px',
                              borderRadius: '50%',
                              backgroundColor: isActive ? 'var(--status-success)' : 'var(--text-muted)',
                            }}
                          />
                          {isActive ? 'Active' : 'Disabled'}
                        </span>
                      </div>

                      {plugin.description && (
                        <p style={{ fontSize: '12px', color: 'var(--text-secondary)', marginTop: '2px' }}>
                          {plugin.description}
                        </p>
                      )}

                      <div
                        style={{
                          fontSize: '11px',
                          fontFamily: 'var(--font-mono)',
                          color: 'var(--text-muted)',
                          marginTop: '4px',
                        }}
                      >
                        {plugin.transport === 'stdio'
                          ? `${plugin.command || ''} ${(plugin.args || []).join(' ')}`
                          : plugin.url}
                      </div>
                    </div>
                  </div>

                  {/* Actions */}
                  <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                    <button
                      type="button"
                      onClick={() => onTogglePlugin(plugin.id)}
                      style={{
                        padding: '4px 8px',
                        borderRadius: 'var(--radius-sm)',
                        backgroundColor: 'var(--bg-main)',
                        border: '1px solid var(--border-subtle)',
                        color: isActive ? 'var(--text-secondary)' : 'var(--text-muted)',
                        fontSize: '11px',
                        cursor: 'pointer',
                        display: 'flex',
                        alignItems: 'center',
                        gap: '4px',
                      }}
                      title={isActive ? 'Disable plugin' : 'Enable plugin'}
                    >
                      <Power size={11} color={isActive ? 'var(--status-success)' : 'var(--text-muted)'} />
                      <span>{isActive ? 'Disable' : 'Enable'}</span>
                    </button>

                    <button
                      type="button"
                      onClick={() => onDeletePlugin(plugin.id)}
                      style={{
                        padding: '4px 6px',
                        borderRadius: 'var(--radius-sm)',
                        backgroundColor: 'transparent',
                        border: '1px solid transparent',
                        color: 'var(--text-muted)',
                        cursor: 'pointer',
                      }}
                      onMouseEnter={(e) => {
                        e.currentTarget.style.color = 'var(--status-danger)';
                      }}
                      onMouseLeave={(e) => {
                        e.currentTarget.style.color = 'var(--text-muted)';
                      }}
                      title="Delete plugin"
                    >
                      <Trash2 size={13} />
                    </button>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>

      <AddMcpModal
        isOpen={isAddModalOpen}
        onClose={() => setIsAddModalOpen(false)}
        onAdd={onAddPlugin}
      />
    </div>
  );
};
