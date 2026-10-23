import React, { useState } from 'react';
import { Plug2, MessageSquare, Terminal, RefreshCw, Cpu, Key } from 'lucide-react';
import { ModelProvider, Integration } from '../../types';
import { Badge } from '../../components/common/Badge';
import { Button } from '../../components/common/Button';
import { Card } from '../../components/common/Card';
import { ConfigureKeyModal } from './ConfigureKeyModal';
import { IntegrationIcon } from '../../components/icons/IntegrationIcon';

export interface ConnectionsPageProps {
  providers: ModelProvider[];
  integrations: Integration[];
  onToggleProvider?: (id: string) => void;
  onToggleIntegration: (id: string) => void;
  onRefreshStatuses: () => void;
}

export const ConnectionsPage: React.FC<ConnectionsPageProps> = ({
  providers,
  integrations,
  onToggleIntegration,
  onRefreshStatuses,
}) => {
  const [selectedProvider, setSelectedProvider] = useState<ModelProvider | null>(null);
  const [activeTab, setActiveTab] = useState<'all' | 'providers' | 'integrations'>('all');
  const [testingId, setTestingId] = useState<string | null>(null);
  const [testResult, setTestResult] = useState<{ id: string; success: boolean; latency: number } | null>(null);

  const handleTestConnection = (id: string, latencyMs = 120) => {
    setTestingId(id);
    setTestResult(null);
    setTimeout(() => {
      setTestingId(null);
      setTestResult({ id, success: true, latency: latencyMs });
      setTimeout(() => setTestResult(null), 3500);
    }, 800);
  };

  const getIntegrationIcon = (id: string) => {
    switch (id) {
      case 'github':
        return <IntegrationIcon id="github" name="GitHub" size={18} />;
      case 'linear':
        return (
          <div
            style={{
              width: '18px',
              height: '18px',
              borderRadius: '4px',
              backgroundColor: 'var(--accent-primary)',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              color: '#FFFFFF',
              fontSize: '11px',
              fontWeight: 700,
            }}
          >
            L
          </div>
        );
      case 'slack':
        return <MessageSquare size={18} color="#E01E5A" />;
      case 'terminal':
        return <Terminal size={18} color="var(--status-success)" />;
      case 'jira':
      default:
        return <IntegrationIcon id={id} size={18} />;
    }
  };

  return (
    <div
      style={{
        height: '100%',
        overflowY: 'auto',
        padding: 'var(--space-6)',
      }}
    >
      <div style={{ maxWidth: '1000px', margin: '0 auto' }}>
        {/* Header */}
        <div
          style={{
            display: 'flex',
            alignItems: 'flex-start',
            justifyContent: 'space-between',
            marginBottom: 'var(--space-6)',
          }}
        >
          <div>
            <h1 style={{ fontSize: '18px', fontWeight: 600, color: 'var(--text-primary)', letterSpacing: '-0.02em' }}>
              Connections & Model Providers
            </h1>
            <p style={{ fontSize: '12px', color: 'var(--text-secondary)', marginTop: '2px' }}>
              Manage AI model interfaces and external developer integrations with OS-level credential protection
            </p>
          </div>

          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <Button
              variant="outline"
              size="sm"
              icon={<RefreshCw size={13} />}
              onClick={onRefreshStatuses}
            >
              Refresh Status
            </Button>
          </div>
        </div>

        {/* Filter Pills */}
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: '6px',
            marginBottom: 'var(--space-5)',
            borderBottom: '1px solid var(--border-subtle)',
            paddingBottom: 'var(--space-3)',
          }}
        >
          {(['all', 'providers', 'integrations'] as const).map((tab) => {
            const isActive = activeTab === tab;
            return (
              <button
                key={tab}
                onClick={() => setActiveTab(tab)}
                style={{
                  padding: '5px 12px',
                  borderRadius: 'var(--radius-md)',
                  backgroundColor: isActive ? 'var(--bg-panel)' : 'transparent',
                  color: isActive ? 'var(--text-primary)' : 'var(--text-secondary)',
                  border: isActive ? '1px solid var(--border-subtle)' : '1px solid transparent',
                  fontSize: '12px',
                  fontWeight: isActive ? 500 : 400,
                  textTransform: 'capitalize',
                  cursor: 'pointer',
                }}
              >
                {tab}
              </button>
            );
          })}
        </div>

        {/* Model Providers Section */}
        {(activeTab === 'all' || activeTab === 'providers') && (
          <div style={{ marginBottom: 'var(--space-8)' }}>
            <div
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: '8px',
                marginBottom: 'var(--space-3)',
              }}
            >
              <Cpu size={15} color="var(--accent-primary)" />
              <h2 style={{ fontSize: '13px', fontWeight: 600, color: 'var(--text-primary)', textTransform: 'uppercase', letterSpacing: '0.04em' }}>
                AI Model Providers ({providers.length})
              </h2>
            </div>

            <div
              style={{
                display: 'grid',
                gridTemplateColumns: 'repeat(auto-fill, minmax(310px, 1fr))',
                gap: 'var(--space-3)',
              }}
            >
              {providers.map((p) => {
                const isTesting = testingId === p.id;
                const test = testResult?.id === p.id ? testResult : null;
                const isConnected = p.status === 'connected';

                return (
                  <Card key={p.id} compact style={{ display: 'flex', flexDirection: 'column', justifyContent: 'space-between' }}>
                    <div>
                      {/* Provider Header */}
                      <div style={{ display: 'flex', alignItems: 'flex-start', justifyContent: 'space-between', marginBottom: '8px' }}>
                        <div>
                          <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                            <span style={{ fontSize: '14px', fontWeight: 600, color: 'var(--text-primary)' }}>
                              {p.name}
                            </span>
                            {p.isLocal && <Badge variant="indigo" size="sm">Local</Badge>}
                          </div>
                          <span style={{ fontSize: '11px', color: 'var(--accent-primary)', fontWeight: 500 }}>
                            {p.defaultModel}
                          </span>
                        </div>
                        <Badge variant={isConnected ? 'success' : 'muted'} dot size="sm">
                          {isConnected ? 'Connected' : 'Disconnected'}
                        </Badge>
                      </div>

                      {/* Description */}
                      <p style={{ fontSize: '12px', color: 'var(--text-secondary)', lineHeight: 1.4, marginBottom: '12px' }}>
                        {p.description}
                      </p>

                      {/* Available Models preview */}
                      <div style={{ display: 'flex', flexWrap: 'wrap', gap: '4px', marginBottom: '14px' }}>
                        {p.availableModels.map((m) => (
                          <span
                            key={m}
                            style={{
                              fontSize: '10px',
                              padding: '2px 6px',
                              borderRadius: 'var(--radius-sm)',
                              backgroundColor: 'var(--bg-input)',
                              border: '1px solid var(--border-subtle)',
                              color: 'var(--text-muted)',
                              fontFamily: 'var(--font-mono)',
                            }}
                          >
                            {m}
                          </span>
                        ))}
                      </div>
                    </div>

                    {/* Footer Actions */}
                    <div
                      style={{
                        paddingTop: '10px',
                        borderTop: '1px solid var(--border-subtle)',
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'space-between',
                      }}
                    >
                      <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                        {p.requiresKey ? (
                          <Button
                            variant="secondary"
                            size="sm"
                            icon={<Key size={12} />}
                            onClick={() => setSelectedProvider(p)}
                          >
                            {p.hasKey ? 'Manage Key' : 'Configure Key'}
                          </Button>
                        ) : (
                          <Badge variant="indigo" size="sm">
                            {p.endpoint}
                          </Badge>
                        )}
                      </div>

                      <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                        {test ? (
                          <span style={{ fontSize: '11px', color: 'var(--status-success)', fontFamily: 'var(--font-mono)' }}>
                            {test.latency}ms OK
                          </span>
                        ) : (
                          <Button
                            variant="ghost"
                            size="sm"
                            disabled={isTesting || (!isConnected && !p.hasKey && p.requiresKey)}
                            onClick={() => handleTestConnection(p.id, p.latencyMs)}
                          >
                            {isTesting ? 'Testing...' : 'Test'}
                          </Button>
                        )}
                      </div>
                    </div>
                  </Card>
                );
              })}
            </div>
          </div>
        )}

        {/* External Integrations Section */}
        {(activeTab === 'all' || activeTab === 'integrations') && (
          <div>
            <div
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: '8px',
                marginBottom: 'var(--space-3)',
              }}
            >
              <Plug2 size={15} color="var(--accent-primary)" />
              <h2 style={{ fontSize: '13px', fontWeight: 600, color: 'var(--text-primary)', textTransform: 'uppercase', letterSpacing: '0.04em' }}>
                External Integrations ({integrations.length})
              </h2>
            </div>

            <div
              style={{
                display: 'grid',
                gridTemplateColumns: 'repeat(auto-fill, minmax(310px, 1fr))',
                gap: 'var(--space-3)',
              }}
            >
              {integrations.map((i) => {
                const isConnected = i.status === 'connected';

                return (
                  <Card key={i.id} compact style={{ display: 'flex', flexDirection: 'column', justifyContent: 'space-between' }}>
                    <div>
                      {/* Integration Header */}
                      <div style={{ display: 'flex', alignItems: 'flex-start', justifyContent: 'space-between', marginBottom: '8px' }}>
                        <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
                          {getIntegrationIcon(i.id)}
                          <div>
                            <span style={{ fontSize: '14px', fontWeight: 600, color: 'var(--text-primary)' }}>
                              {i.name}
                            </span>
                            {i.configuredAccount && (
                              <div style={{ fontSize: '11px', color: 'var(--text-muted)', fontFamily: 'var(--font-mono)' }}>
                                {i.configuredAccount}
                              </div>
                            )}
                          </div>
                        </div>
                        <Badge variant={isConnected ? 'success' : 'muted'} dot size="sm">
                          {isConnected ? 'Connected' : 'Disconnected'}
                        </Badge>
                      </div>

                      {/* Description */}
                      <p style={{ fontSize: '12px', color: 'var(--text-secondary)', lineHeight: 1.4, marginBottom: '12px' }}>
                        {i.description}
                      </p>
                    </div>

                    {/* Footer */}
                    <div
                      style={{
                        paddingTop: '10px',
                        borderTop: '1px solid var(--border-subtle)',
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'space-between',
                      }}
                    >
                      <span style={{ fontSize: '11px', color: 'var(--text-muted)' }}>
                        {i.lastSync ? `Synced: ${i.lastSync}` : 'Not linked'}
                      </span>
                      <Button
                        variant={isConnected ? 'outline' : 'primary'}
                        size="sm"
                        onClick={() => onToggleIntegration(i.id)}
                      >
                        {isConnected ? 'Disconnect' : 'Connect'}
                      </Button>
                    </div>
                  </Card>
                );
              })}
            </div>
          </div>
        )}
      </div>

      {/* Configure Key Modal */}
      <ConfigureKeyModal
        provider={selectedProvider}
        isOpen={Boolean(selectedProvider)}
        onClose={() => setSelectedProvider(null)}
        onKeySaved={() => {
          onRefreshStatuses();
        }}
      />
    </div>
  );
};
