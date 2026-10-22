import React, { useState } from 'react';
import { Modal } from '../../components/common/Modal';
import { Button } from '../../components/common/Button';
import { Input } from '../../components/common/Input';
import { McpPlugin } from '../../types';

export interface AddMcpModalProps {
  isOpen: boolean;
  onClose: () => void;
  onAdd: (plugin: Omit<McpPlugin, 'id' | 'createdAt' | 'status'>) => void;
}

export const AddMcpModal: React.FC<AddMcpModalProps> = ({ isOpen, onClose, onAdd }) => {
  const [name, setName] = useState('');
  const [transport, setTransport] = useState<'stdio' | 'sse'>('stdio');
  const [command, setCommand] = useState('');
  const [args, setArgs] = useState('');
  const [url, setUrl] = useState('');
  const [description, setDescription] = useState('');
  const [error, setError] = useState('');

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) {
      setError('Server name is required');
      return;
    }
    if (transport === 'stdio' && !command.trim()) {
      setError('Executable command is required for stdio transport');
      return;
    }
    if (transport === 'sse' && !url.trim()) {
      setError('Endpoint URL is required for SSE transport');
      return;
    }

    const parsedArgs = args
      .split(' ')
      .map((s) => s.trim())
      .filter(Boolean);

    onAdd({
      name: name.trim(),
      transport,
      command: transport === 'stdio' ? command.trim() : undefined,
      args: transport === 'stdio' && parsedArgs.length > 0 ? parsedArgs : undefined,
      url: transport === 'sse' ? url.trim() : undefined,
      description: description.trim() || undefined,
    });

    setName('');
    setCommand('');
    setArgs('');
    setUrl('');
    setDescription('');
    setError('');
    onClose();
  };

  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      title="Add MCP Server Integration"
      description="Configure a local Model Context Protocol server via stdio or Server-Sent Events (SSE)."
      footer={
        <>
          <Button variant="ghost" size="sm" onClick={onClose}>
            Cancel
          </Button>
          <Button variant="primary" size="sm" onClick={handleSubmit}>
            Save Server
          </Button>
        </>
      }
    >
      <form onSubmit={handleSubmit} style={{ display: 'flex', flexDirection: 'column', gap: '14px' }}>
        {error && (
          <div
            style={{
              padding: '6px 10px',
              backgroundColor: 'var(--status-danger-bg)',
              border: '1px solid var(--status-danger-border)',
              borderRadius: 'var(--radius-md)',
              fontSize: '12px',
              color: 'var(--status-danger)',
            }}
          >
            {error}
          </div>
        )}

        <Input
          label="Server Name"
          placeholder="e.g. local-filesystem, sqlite-service"
          value={name}
          onChange={(e) => {
            setName(e.target.value);
            setError('');
          }}
        />

        <div>
          <label style={{ fontSize: '12px', fontWeight: 500, color: 'var(--text-secondary)', display: 'block', marginBottom: '6px' }}>
            Transport Protocol
          </label>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '8px' }}>
            <button
              type="button"
              onClick={() => setTransport('stdio')}
              style={{
                height: '32px',
                borderRadius: 'var(--radius-md)',
                backgroundColor: transport === 'stdio' ? 'var(--bg-input-focus)' : 'var(--bg-input)',
                border: `1px solid ${transport === 'stdio' ? 'var(--accent-primary)' : 'var(--border-subtle)'}`,
                color: transport === 'stdio' ? 'var(--text-primary)' : 'var(--text-secondary)',
                fontSize: '12px',
                fontWeight: transport === 'stdio' ? 500 : 400,
                cursor: 'pointer',
              }}
            >
              stdio (Local Process)
            </button>
            <button
              type="button"
              onClick={() => setTransport('sse')}
              style={{
                height: '32px',
                borderRadius: 'var(--radius-md)',
                backgroundColor: transport === 'sse' ? 'var(--bg-input-focus)' : 'var(--bg-input)',
                border: `1px solid ${transport === 'sse' ? 'var(--accent-primary)' : 'var(--border-subtle)'}`,
                color: transport === 'sse' ? 'var(--text-primary)' : 'var(--text-secondary)',
                fontSize: '12px',
                fontWeight: transport === 'sse' ? 500 : 400,
                cursor: 'pointer',
              }}
            >
              SSE (HTTP Stream)
            </button>
          </div>
        </div>

        {transport === 'stdio' ? (
          <>
            <Input
              label="Executable / Command"
              placeholder="e.g. npx, uvx, python"
              value={command}
              onChange={(e) => setCommand(e.target.value)}
            />
            <Input
              label="Command Arguments"
              placeholder="e.g. -y @modelcontextprotocol/server-filesystem D:/workspace"
              value={args}
              onChange={(e) => setArgs(e.target.value)}
            />
          </>
        ) : (
          <Input
            label="SSE Endpoint URL"
            placeholder="http://localhost:8000/sse"
            value={url}
            onChange={(e) => setUrl(e.target.value)}
          />
        )}

        <Input
          label="Description (Optional)"
          placeholder="Brief description of the tools provided by this server"
          value={description}
          onChange={(e) => setDescription(e.target.value)}
        />
      </form>
    </Modal>
  );
};
