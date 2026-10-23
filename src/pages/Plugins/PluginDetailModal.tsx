import React, { useState, useEffect } from 'react';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import {
  X,
  Check,
  AlertCircle,
  Eye,
  EyeOff,
  Terminal,
  Globe,
  Cpu,
  Shield,
  Layers,
  Activity,
  Play,
  RotateCw,
  PowerOff,
  Trash2,
  ExternalLink,
} from 'lucide-react';
import {
  CatalogEntry,
  PluginMetadata,
  PluginConfig,
  PluginKind,
  PluginStatus,
  TestConnectionResult,
} from '../../types';
import { pluginStore } from '../../services/pluginStore';
import { IntegrationIcon } from '../../components/icons/IntegrationIcon';

export interface PluginDetailModalProps {
  isOpen: boolean;
  onClose: () => void;
  catalogEntry?: CatalogEntry | null;
  pluginMetadata?: PluginMetadata | null;
  onRefresh?: () => void;
}

export const PluginDetailModal: React.FC<PluginDetailModalProps> = ({
  isOpen,
  onClose,
  catalogEntry,
  pluginMetadata,
  onRefresh,
}) => {
  const [command, setCommand] = useState('');
  const [args, setArgs] = useState('');
  const [url, setUrl] = useState('');
  const [approvedPaths, setApprovedPaths] = useState('');
  const [secretKey, setSecretKey] = useState('');
  const [showSecret, setShowSecret] = useState(false);
  const [isTesting, setIsTesting] = useState(false);
  const [isConnecting, setIsConnecting] = useState(false);
  const [testResult, setTestResult] = useState<TestConnectionResult | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [saveSuccess, setSaveSuccess] = useState(false);

  // Sync state whenever selected entry changes
  useEffect(() => {
    if (!isOpen) return;

    setActionError(null);
    setTestResult(null);
    setSaveSuccess(false);

    if (pluginMetadata) {
      setCommand(pluginMetadata.settings.command || '');
      setArgs(pluginMetadata.settings.args || '');
      setUrl(pluginMetadata.settings.url || '');
      setApprovedPaths((pluginMetadata.approved_paths || []).join(', '));
      setSecretKey('');
    } else if (catalogEntry) {
      setCommand(catalogEntry.default_command || '');
      setArgs((catalogEntry.default_args || []).join(' '));
      setUrl(catalogEntry.default_url || '');
      setApprovedPaths('d:\\Projects\\Desktop\\Aven Agent');
      setSecretKey('');
    }
  }, [isOpen, catalogEntry, pluginMetadata]);

  if (!isOpen || (!catalogEntry && !pluginMetadata)) {
    return null;
  }

  const kind: PluginKind = catalogEntry?.kind || pluginMetadata?.kind || 'mcp';
  const name = catalogEntry?.name || pluginMetadata?.name || 'Integration';
  const id = catalogEntry?.id || pluginMetadata?.id || '';
  const currentStatus: PluginStatus = pluginMetadata?.status || 'not_configured';
  const isImplemented = catalogEntry?.is_implemented !== false;

  const getKindBadge = () => {
    switch (kind) {
      case 'mcp':
        return { label: 'MCP SERVER', color: '#7C7CF8', bg: 'rgba(124, 124, 248, 0.1)', icon: <Terminal size={12} /> };
      case 'api':
        return { label: 'API INTEGRATION', color: '#38BDF8', bg: 'rgba(56, 189, 248, 0.1)', icon: <Globe size={12} /> };
      case 'native':
        return { label: 'NATIVE CAPABILITY', color: '#34D399', bg: 'rgba(52, 211, 153, 0.1)', icon: <Cpu size={12} /> };
    }
  };

  const getStatusBadge = () => {
    switch (currentStatus) {
      case 'connected':
        return { label: 'Connected', color: '#34D399', bg: 'rgba(52, 211, 153, 0.12)', dot: true };
      case 'connecting':
        return { label: 'Connecting...', color: '#FBBF24', bg: 'rgba(251, 191, 36, 0.12)', dot: true };
      case 'disconnected':
        return { label: 'Disconnected', color: '#A5A6B0', bg: 'rgba(165, 166, 176, 0.12)', dot: false };
      case 'needs_permission':
        return { label: 'Needs Setup', color: '#FBBF24', bg: 'rgba(251, 191, 36, 0.12)', dot: false };
      case 'error':
        return { label: 'Error', color: '#F87171', bg: 'rgba(248, 113, 113, 0.12)', dot: false };
      case 'unsupported':
        return { label: 'Unsupported OS', color: '#6B7280', bg: 'rgba(107, 114, 128, 0.12)', dot: false };
      default:
        return { label: 'Not Configured', color: '#7C7CF8', bg: 'rgba(124, 124, 248, 0.12)', dot: false };
    }
  };

  const handleSaveConfig = async (andConnect = false) => {
    setActionError(null);
    setSaveSuccess(false);

    try {
      const parsedArgs = args
        .split(' ')
        .map((s) => s.trim())
        .filter(Boolean);

      const pathsList = approvedPaths
        .split(',')
        .map((s) => s.trim())
        .filter(Boolean);

      const isAlreadyActive = pluginMetadata ? (pluginMetadata.status === 'connected' || pluginMetadata.status === 'connecting') : false;
      const config: PluginConfig = {
        id,
        kind,
        name,
        enabled: andConnect || isAlreadyActive,
        transport: kind === 'mcp' ? (url.trim() ? 'http' : 'stdio') : undefined,
        command: command.trim() || undefined,
        args: parsedArgs.length > 0 ? parsedArgs : undefined,
        url: url.trim() || undefined,
        approved_paths: pathsList.length > 0 ? pathsList : undefined,
        settings: {
          command: command.trim(),
          args: args.trim(),
          url: url.trim(),
        },
        last_status: andConnect ? 'connecting' : (pluginMetadata?.status || 'disconnected'),
      };

      await pluginStore.savePlugin(config, secretKey.trim() || undefined);
      setSaveSuccess(true);
      setTimeout(() => setSaveSuccess(false), 3000);

      if (andConnect) {
        setIsConnecting(true);
        try {
          await pluginStore.connect(id);
        } catch (connErr) {
          setActionError(String(connErr));
        } finally {
          setIsConnecting(false);
        }
      }

      if (onRefresh) onRefresh();
    } catch (err) {
      setActionError(String(err));
    }
  };

  const handleTestConnection = async () => {
    setIsTesting(true);
    setActionError(null);
    setTestResult(null);

    try {
      // First save any updated config/secret
      if (secretKey.trim()) {
        await handleSaveConfig(false);
      }
      const res = await pluginStore.testConnection(id);
      setTestResult(res);
      if (onRefresh) onRefresh();
    } catch (err) {
      setActionError(String(err));
    } finally {
      setIsTesting(false);
    }
  };

  const handleDisconnect = async () => {
    setActionError(null);
    try {
      await pluginStore.disconnect(id);
      if (onRefresh) onRefresh();
    } catch (err) {
      setActionError(String(err));
    }
  };

  const handleDelete = async () => {
    if (!window.confirm(`Are you sure you want to remove configuration and stored credentials for '${name}'?`)) {
      return;
    }
    try {
      await pluginStore.deletePlugin(id);
      if (onRefresh) onRefresh();
      onClose();
    } catch (err) {
      setActionError(String(err));
    }
  };

  const kindBadge = getKindBadge();
  const statusBadge = getStatusBadge();

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        backgroundColor: 'rgba(0, 0, 0, 0.65)',
        backdropFilter: 'blur(3px)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        zIndex: 1000,
        padding: '24px',
      }}
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        style={{
          width: '100%',
          maxWidth: '720px',
          maxHeight: '90vh',
          backgroundColor: '#1E1F24',
          border: '1px solid #34353C',
          borderRadius: '10px',
          display: 'flex',
          flexDirection: 'column',
          overflow: 'hidden',
          boxShadow: '0 20px 40px rgba(0,0,0,0.5)',
        }}
      >
        {/* Modal Header */}
        <div
          style={{
            padding: '16px 20px',
            borderBottom: '1px solid #34353C',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            backgroundColor: '#25262C',
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
            <span
              style={{
                display: 'inline-flex',
                alignItems: 'center',
                gap: '5px',
                padding: '2px 8px',
                borderRadius: '4px',
                fontSize: '10px',
                fontFamily: 'var(--font-mono)',
                fontWeight: 600,
                color: kindBadge.color,
                backgroundColor: kindBadge.bg,
                border: `1px solid ${kindBadge.color}40`,
              }}
            >
              {kindBadge.icon}
              {kindBadge.label}
            </span>

            <span
              style={{
                display: 'inline-flex',
                alignItems: 'center',
                gap: '5px',
                padding: '2px 8px',
                borderRadius: '4px',
                fontSize: '11px',
                fontWeight: 500,
                color: statusBadge.color,
                backgroundColor: statusBadge.bg,
              }}
            >
              {statusBadge.dot && (
                <span
                  style={{
                    width: '6px',
                    height: '6px',
                    borderRadius: '50%',
                    backgroundColor: statusBadge.color,
                    boxShadow: `0 0 6px ${statusBadge.color}`,
                  }}
                />
              )}
              {statusBadge.label}
            </span>
          </div>

          <button
            type="button"
            onClick={onClose}
            style={{
              background: 'transparent',
              border: 'none',
              color: '#A5A6B0',
              cursor: 'pointer',
              padding: '4px',
              borderRadius: '4px',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
            }}
          >
            <X size={16} />
          </button>
        </div>

        {/* Modal Scrollable Body */}
        <div style={{ flex: 1, overflowY: 'auto', padding: '20px' }}>
          {/* Title & Category */}
          <div style={{ marginBottom: '16px' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
              <div
                style={{
                  width: '32px',
                  height: '32px',
                  borderRadius: '6px',
                  backgroundColor: '#25262C',
                  border: '1px solid #34353C',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  flexShrink: 0,
                }}
              >
                <IntegrationIcon id={id} name={name} type={kind} size={18} />
              </div>
              <h2 style={{ fontSize: '18px', fontWeight: 600, color: '#F4F4F5', margin: 0 }}>
                {name}
              </h2>
            </div>
            <div style={{ fontSize: '12px', color: '#A5A6B0', marginTop: '6px', display: 'flex', gap: '8px' }}>
              <span>Category: <strong>{catalogEntry?.category || 'General'}</strong></span>
              <span>•</span>
              <span style={{ fontFamily: 'var(--font-mono)' }}>ID: {id}</span>
            </div>
          </div>

          {/* Action error banner */}
          {actionError && (
            <div
              style={{
                padding: '10px 14px',
                borderRadius: '6px',
                backgroundColor: 'rgba(239, 68, 68, 0.1)',
                border: '1px solid rgba(239, 68, 68, 0.25)',
                color: '#F87171',
                fontSize: '12px',
                marginBottom: '16px',
                display: 'flex',
                alignItems: 'flex-start',
                gap: '8px',
              }}
            >
              <AlertCircle size={15} style={{ flexShrink: 0, marginTop: '2px' }} />
              <div>
                <strong>Operation Failed:</strong> {actionError}
              </div>
            </div>
          )}

          {/* Save success banner */}
          {saveSuccess && (
            <div
              style={{
                padding: '8px 12px',
                borderRadius: '6px',
                backgroundColor: 'rgba(52, 211, 153, 0.1)',
                border: '1px solid rgba(52, 211, 153, 0.25)',
                color: '#34D399',
                fontSize: '12px',
                marginBottom: '16px',
                display: 'flex',
                alignItems: 'center',
                gap: '6px',
              }}
            >
              <Check size={14} />
              <span>Configuration saved successfully. Credentials stored in OS Credential Manager.</span>
            </div>
          )}

          {/* Test Result Banner */}
          {testResult && (
            <div
              style={{
                padding: '10px 14px',
                borderRadius: '6px',
                backgroundColor: testResult.success ? 'rgba(52, 211, 153, 0.1)' : 'rgba(239, 68, 68, 0.1)',
                border: `1px solid ${testResult.success ? 'rgba(52, 211, 153, 0.25)' : 'rgba(239, 68, 68, 0.25)'}`,
                color: testResult.success ? '#34D399' : '#F87171',
                fontSize: '12px',
                marginBottom: '16px',
                display: 'flex',
                alignItems: 'flex-start',
                gap: '8px',
              }}
            >
              {testResult.success ? <Check size={15} style={{ flexShrink: 0, marginTop: '2px' }} /> : <AlertCircle size={15} style={{ flexShrink: 0, marginTop: '2px' }} />}
              <div>
                <strong>{testResult.success ? 'Connection Successful' : 'Connection Failed'}:</strong> {testResult.message}
                {testResult.latency_ms > 0 && (
                  <span style={{ marginLeft: '8px', opacity: 0.8, fontFamily: 'var(--font-mono)' }}>
                    ({testResult.latency_ms}ms)
                  </span>
                )}
              </div>
            </div>
          )}

          {/* Description rendered as safely sanitized Markdown */}
          <div
            style={{
              padding: '12px 14px',
              backgroundColor: '#25262C',
              border: '1px solid #34353C',
              borderRadius: '6px',
              fontSize: '13px',
              color: '#D4D4D8',
              lineHeight: 1.6,
              marginBottom: '18px',
            }}
          >
            <ReactMarkdown remarkPlugins={[remarkGfm]}>
              {catalogEntry?.description || pluginMetadata?.error_message || 'No description available.'}
            </ReactMarkdown>
          </div>

          {/* Requirements & Setup Instructions */}
          {catalogEntry?.requirements && (
            <div style={{ marginBottom: '18px' }}>
              <div style={{ fontSize: '11px', fontWeight: 600, color: '#A5A6B0', textTransform: 'uppercase', letterSpacing: '0.05em', marginBottom: '6px' }}>
                Requirements & Setup
              </div>
              <div style={{ fontSize: '12px', color: '#E4E4E7', backgroundColor: '#1A1B20', padding: '10px 12px', borderRadius: '6px', border: '1px solid #2D2E35' }}>
                <div style={{ marginBottom: '4px' }}>
                  <span style={{ color: '#A5A6B0' }}>Prerequisites: </span>
                  {catalogEntry.requirements}
                </div>
                {catalogEntry.setup_instructions && (
                  <div>
                    <span style={{ color: '#A5A6B0' }}>Instructions: </span>
                    {catalogEntry.setup_instructions}
                  </div>
                )}
              </div>
            </div>
          )}

          {/* Permissions & Security Notice */}
          <div
            style={{
              padding: '10px 12px',
              backgroundColor: 'rgba(124, 124, 248, 0.05)',
              border: '1px solid rgba(124, 124, 248, 0.2)',
              borderRadius: '6px',
              marginBottom: '20px',
              display: 'flex',
              alignItems: 'flex-start',
              gap: '10px',
            }}
          >
            <Shield size={16} color="#7C7CF8" style={{ flexShrink: 0, marginTop: '2px' }} />
            <div style={{ fontSize: '12px', color: '#D1D1DF', lineHeight: 1.5 }}>
              <strong style={{ color: '#7C7CF8' }}>Security & Permissions Notice:</strong>{' '}
              {catalogEntry?.permissions_notice ||
                'Capabilities are governed by strict backend boundary enforcement. Secrets are stored in OS Credential Manager.'}
            </div>
          </div>

          {/* Discovered / Supported Capabilities */}
          <div style={{ marginBottom: '22px' }}>
            <div style={{ fontSize: '11px', fontWeight: 600, color: '#A5A6B0', textTransform: 'uppercase', letterSpacing: '0.05em', marginBottom: '8px', display: 'flex', alignItems: 'center', gap: '6px' }}>
              <Layers size={13} />
              <span>Supported Capabilities & Tools ({pluginMetadata?.discovered_tools?.length || catalogEntry?.supported_capabilities?.length || 0})</span>
            </div>

            {pluginMetadata?.discovered_tools && pluginMetadata.discovered_tools.length > 0 ? (
              <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
                {pluginMetadata.discovered_tools.map((t) => (
                  <div
                    key={t.name}
                    style={{
                      padding: '8px 12px',
                      backgroundColor: '#25262C',
                      border: '1px solid #34353C',
                      borderRadius: '6px',
                    }}
                  >
                    <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                      <span style={{ fontFamily: 'var(--font-mono)', fontSize: '12.5px', fontWeight: 600, color: '#7C7CF8' }}>
                        {t.name}
                      </span>
                      <span style={{ fontSize: '10px', color: '#34D399', backgroundColor: 'rgba(52,211,153,0.1)', padding: '1px 6px', borderRadius: '3px' }}>
                        Discovered
                      </span>
                    </div>
                    {t.description && (
                      <p style={{ fontSize: '11.5px', color: '#A5A6B0', margin: '4px 0 0 0' }}>
                        {t.description}
                      </p>
                    )}
                  </div>
                ))}
              </div>
            ) : (
              <div style={{ display: 'flex', flexWrap: 'wrap', gap: '6px' }}>
                {(catalogEntry?.supported_capabilities || []).map((cap) => (
                  <span
                    key={cap}
                    style={{
                      padding: '4px 8px',
                      borderRadius: '4px',
                      backgroundColor: '#25262C',
                      border: '1px solid #34353C',
                      fontSize: '11.5px',
                      fontFamily: 'var(--font-mono)',
                      color: '#E4E4E7',
                    }}
                  >
                    {cap}
                  </span>
                ))}
              </div>
            )}
          </div>

          {/* Configuration Form */}
          {isImplemented && (
            <div
              style={{
                padding: '16px',
                backgroundColor: '#212227',
                border: '1px solid #34353C',
                borderRadius: '8px',
                marginBottom: '20px',
              }}
            >
              <h3 style={{ fontSize: '13px', fontWeight: 600, color: '#F4F4F5', margin: '0 0 14px 0' }}>
                Integration Configuration
              </h3>

              {/* Stdio command & args */}
              {kind === 'mcp' && (
                <>
                  <div style={{ marginBottom: '12px' }}>
                    <label style={{ display: 'block', fontSize: '11px', color: '#A5A6B0', marginBottom: '4px' }}>
                      Executable Command
                    </label>
                    <input
                      type="text"
                      value={command}
                      onChange={(e) => setCommand(e.target.value)}
                      placeholder="e.g. npx, uvx, python"
                      style={{
                        width: '100%',
                        padding: '7px 10px',
                        backgroundColor: '#17181D',
                        border: '1px solid #34353C',
                        borderRadius: '5px',
                        color: '#F4F4F5',
                        fontSize: '12px',
                        fontFamily: 'var(--font-mono)',
                        outline: 'none',
                      }}
                    />
                  </div>

                  <div style={{ marginBottom: '12px' }}>
                    <label style={{ display: 'block', fontSize: '11px', color: '#A5A6B0', marginBottom: '4px' }}>
                      Command Arguments (space separated)
                    </label>
                    <input
                      type="text"
                      value={args}
                      onChange={(e) => setArgs(e.target.value)}
                      placeholder="e.g. -y @modelcontextprotocol/server-filesystem"
                      style={{
                        width: '100%',
                        padding: '7px 10px',
                        backgroundColor: '#17181D',
                        border: '1px solid #34353C',
                        borderRadius: '5px',
                        color: '#F4F4F5',
                        fontSize: '12px',
                        fontFamily: 'var(--font-mono)',
                        outline: 'none',
                      }}
                    />
                  </div>
                </>
              )}

              {/* Scoped Approved Directories (for Filesystem or Native) */}
              {(id === 'mcp_filesystem' || id === 'native_fs') && (
                <div style={{ marginBottom: '12px' }}>
                  <label style={{ display: 'block', fontSize: '11px', color: '#A5A6B0', marginBottom: '4px' }}>
                    Approved Directory Scope (Comma-separated)
                  </label>
                  <input
                    type="text"
                    value={approvedPaths}
                    onChange={(e) => setApprovedPaths(e.target.value)}
                    placeholder="d:\Projects\Desktop\Aven Agent"
                    style={{
                      width: '100%',
                      padding: '7px 10px',
                      backgroundColor: '#17181D',
                      border: '1px solid #34353C',
                      borderRadius: '5px',
                      color: '#F4F4F5',
                      fontSize: '12px',
                      fontFamily: 'var(--font-mono)',
                      outline: 'none',
                    }}
                  />
                  <span style={{ fontSize: '10.5px', color: '#71717A', marginTop: '3px', display: 'block' }}>
                    Path containment is enforced in Rust. Any operation outside these paths is rejected.
                  </span>
                </div>
              )}

              {/* Secret Key / API Key for API integration or GitHub MCP */}
              {(id === 'api_gemini_search' || id === 'mcp_github' || id === 'mcp_custom') && (
                <div style={{ marginBottom: '12px' }}>
                  <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '4px' }}>
                    <label style={{ fontSize: '11px', color: '#A5A6B0' }}>
                      {id === 'api_gemini_search' ? 'Google Gemini API Key' : 'Secret Token / API Key'}
                    </label>
                    {pluginMetadata?.has_secret && (
                      <span style={{ fontSize: '10px', color: '#34D399' }}>
                        Key securely stored in Windows Credential Manager
                      </span>
                    )}
                  </div>
                  <div style={{ position: 'relative' }}>
                    <input
                      type={showSecret ? 'text' : 'password'}
                      value={secretKey}
                      onChange={(e) => setSecretKey(e.target.value)}
                      placeholder={pluginMetadata?.has_secret ? '•••••••••••••••••••••••••••••••• (Leave blank to keep current)' : 'Enter API Key...'}
                      style={{
                        width: '100%',
                        padding: '7px 34px 7px 10px',
                        backgroundColor: '#17181D',
                        border: '1px solid #34353C',
                        borderRadius: '5px',
                        color: '#F4F4F5',
                        fontSize: '12px',
                        fontFamily: 'var(--font-mono)',
                        outline: 'none',
                      }}
                    />
                    <button
                      type="button"
                      onClick={() => setShowSecret(!showSecret)}
                      style={{
                        position: 'absolute',
                        right: '8px',
                        top: '50%',
                        transform: 'translateY(-50%)',
                        background: 'transparent',
                        border: 'none',
                        color: '#71717A',
                        cursor: 'pointer',
                        padding: '2px',
                      }}
                    >
                      {showSecret ? <EyeOff size={14} /> : <Eye size={14} />}
                    </button>
                  </div>
                  {id === 'api_gemini_search' && (
                    <div style={{ fontSize: '11px', color: '#71717A', marginTop: '4px' }}>
                      Get your Gemini API Key from{' '}
                      <a
                        href="https://aistudio.google.com/apikey"
                        target="_blank"
                        rel="noreferrer"
                        style={{ color: '#7C7CF8', textDecoration: 'underline' }}
                      >
                        Google AI Studio <ExternalLink size={10} style={{ display: 'inline' }} />
                      </a>. Never committed or stored plaintext.
                    </div>
                  )}
                </div>
              )}
            </div>
          )}

          {!isImplemented && (
            <div
              style={{
                padding: '14px',
                borderRadius: '6px',
                backgroundColor: 'rgba(251, 191, 36, 0.08)',
                border: '1px solid rgba(251, 191, 36, 0.2)',
                color: '#FBBF24',
                fontSize: '12px',
                lineHeight: 1.5,
              }}
            >
              <strong>Integration Planned:</strong> This entry is in the Aven integration catalog but its server implementation is planned for a future release. You cannot fake-connect it until its actual provider implementation is available.
            </div>
          )}
        </div>

        {/* Modal Footer Controls */}
        <div
          style={{
            padding: '14px 20px',
            borderTop: '1px solid #34353C',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            backgroundColor: '#25262C',
          }}
        >
          <div>
            {pluginMetadata && (
              <button
                type="button"
                onClick={handleDelete}
                style={{
                  display: 'inline-flex',
                  alignItems: 'center',
                  gap: '6px',
                  padding: '6px 10px',
                  borderRadius: '5px',
                  backgroundColor: 'transparent',
                  border: '1px solid #34353C',
                  color: '#F87171',
                  fontSize: '11.5px',
                  cursor: 'pointer',
                }}
              >
                <Trash2 size={13} />
                <span>Remove</span>
              </button>
            )}
          </div>

          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <button
              type="button"
              onClick={onClose}
              style={{
                padding: '6px 12px',
                borderRadius: '5px',
                backgroundColor: 'transparent',
                border: '1px solid #34353C',
                color: '#A5A6B0',
                fontSize: '12px',
                cursor: 'pointer',
              }}
            >
              Cancel
            </button>

            {isImplemented && (
              <>
                <button
                  type="button"
                  onClick={handleTestConnection}
                  disabled={isTesting}
                  style={{
                    display: 'inline-flex',
                    alignItems: 'center',
                    gap: '6px',
                    padding: '6px 12px',
                    borderRadius: '5px',
                    backgroundColor: '#1E1F24',
                    border: '1px solid #34353C',
                    color: '#F4F4F5',
                    fontSize: '12px',
                    cursor: isTesting ? 'not-allowed' : 'pointer',
                    opacity: isTesting ? 0.6 : 1,
                  }}
                >
                  <Activity size={13} className={isTesting ? 'animate-spin' : ''} />
                  <span>{isTesting ? 'Testing...' : 'Test Connection'}</span>
                </button>

                {currentStatus === 'connected' ? (
                  <button
                    type="button"
                    onClick={handleDisconnect}
                    style={{
                      display: 'inline-flex',
                      alignItems: 'center',
                      gap: '6px',
                      padding: '6px 14px',
                      borderRadius: '5px',
                      backgroundColor: '#2D1B1B',
                      border: '1px solid #F87171',
                      color: '#F87171',
                      fontSize: '12px',
                      cursor: 'pointer',
                    }}
                  >
                    <PowerOff size={13} />
                    <span>Disconnect</span>
                  </button>
                ) : (
                  <button
                    type="button"
                    onClick={() => handleSaveConfig(true)}
                    disabled={isConnecting}
                    style={{
                      display: 'inline-flex',
                      alignItems: 'center',
                      gap: '6px',
                      padding: '6px 14px',
                      borderRadius: '5px',
                      backgroundColor: '#7C7CF8',
                      border: 'none',
                      color: '#FFFFFF',
                      fontSize: '12px',
                      fontWeight: 500,
                      cursor: isConnecting ? 'not-allowed' : 'pointer',
                      opacity: isConnecting ? 0.7 : 1,
                    }}
                  >
                    {isConnecting ? (
                      <RotateCw size={13} className="animate-spin" />
                    ) : (
                      <Play size={13} />
                    )}
                    <span>{isConnecting ? 'Connecting...' : 'Connect & Enable'}</span>
                  </button>
                )}
              </>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
