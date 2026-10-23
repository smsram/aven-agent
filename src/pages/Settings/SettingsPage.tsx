import React, { useState, useEffect } from 'react';
import {
  Sliders,
  Palette,
  Key,
  Shield,
  Brain,
  Keyboard,
  Save,
  Check,
  Eye,
  EyeOff,
  Folder,
  Trash2,
  X,
  AlertCircle,
  CheckCircle2,
  RefreshCw,
  AlertTriangle,
  Info,
} from 'lucide-react';
import { AppSettings, ShortcutItem } from '../../types';
import { Button } from '../../components/common/Button';
import { Switch } from '../../components/common/Switch';
import {
  secureStore,
  cloudflareService,
  CredentialsStatus,
  OverallTestResult,
} from '../../services/tauriIpc';

export interface SettingsPageProps {
  isOpen: boolean;
  settings: AppSettings;
  onUpdateSettings: (newSettings: AppSettings) => void;
  onClose: () => void;
}

export const SettingsPage: React.FC<SettingsPageProps> = ({
  isOpen,
  settings,
  onUpdateSettings,
  onClose,
}) => {
  const [activeSection, setActiveSection] = useState<
    'general' | 'appearance' | 'api' | 'permissions' | 'memory' | 'shortcuts'
  >('general');

  const [form, setForm] = useState<AppSettings>(settings);
  const [savedSuccess, setSavedSuccess] = useState(false);

  // Cloudflare Workers AI Credential State
  const [accountIdsInput, setAccountIdsInput] = useState('');
  const [apiKeysInput, setApiKeysInput] = useState('');
  const [showApiKeys, setShowApiKeys] = useState(false);
  const [cfStatus, setCfStatus] = useState<CredentialsStatus | null>(null);
  const [cfSaveStatus, setCfSaveStatus] = useState<string | null>(null);
  const [cfValidationErr, setCfValidationErr] = useState<string | null>(null);
  const [isSavingCf, setIsSavingCf] = useState(false);
  const [isTestingCf, setIsTestingCf] = useState(false);
  const [testResults, setTestResults] = useState<OverallTestResult | null>(null);

  useEffect(() => {
    setForm(settings);
  }, [settings]);

  useEffect(() => {
    if (isOpen) {
      cloudflareService.getStatus().then((status) => {
        setCfStatus(status);
      });
      setCfValidationErr(null);
      setCfSaveStatus(null);
    }
  }, [isOpen]);

  // Handle ESC key to close modal
  useEffect(() => {
    if (!isOpen) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  const handleSaveAll = async () => {
    onUpdateSettings(form);
    await secureStore.saveSettings(form);
    setSavedSuccess(true);
    setTimeout(() => setSavedSuccess(false), 2000);
  };

  const parsedAccountIds = accountIdsInput
    .split(',')
    .map((s) => s.trim())
    .filter(Boolean);
  const parsedApiKeys = apiKeysInput
    .split(',')
    .map((s) => s.trim())
    .filter(Boolean);

  const handleSaveCloudflare = async () => {
    setCfValidationErr(null);
    setCfSaveStatus(null);

    if (parsedAccountIds.length === 0 || parsedApiKeys.length === 0) {
      setCfValidationErr('Please enter at least one Account ID and matching API Key.');
      return;
    }

    if (parsedAccountIds.length !== parsedApiKeys.length) {
      setCfValidationErr(
        `Mismatched count: ${parsedAccountIds.length} Account ID(s) provided, but ${parsedApiKeys.length} API Key(s) provided. Each Account ID must have exactly one corresponding API Key.`
      );
      return;
    }

    setIsSavingCf(true);
    try {
      const updated = await cloudflareService.saveCredentials(accountIdsInput, apiKeysInput);
      setCfStatus(updated);
      setAccountIdsInput('');
      setApiKeysInput('');
      setShowApiKeys(false);
      setTestResults(null);
      setCfSaveStatus(
        `Successfully saved ${updated.configured_count} credential pair(s) in Windows Credential Manager.`
      );
      setTimeout(() => setCfSaveStatus(null), 3000);
    } catch (err: any) {
      setCfValidationErr(err.toString());
    } finally {
      setIsSavingCf(false);
    }
  };

  const handleTestConnection = async () => {
    setIsTestingCf(true);
    setCfValidationErr(null);
    setTestResults(null);
    try {
      const res = await cloudflareService.testCredentials();
      setTestResults(res);
    } catch (err: any) {
      setCfValidationErr(`Test connection failed: ${err.toString()}`);
    } finally {
      setIsTestingCf(false);
    }
  };

  const handleClearCloudflare = async () => {
    try {
      await cloudflareService.clearCredentials();
      setCfStatus({
        configured_count: 0,
        pairs: [],
        preferred_index: 0,
        model_id: '@cf/qwen/qwen3.8-27b',
      });
      setAccountIdsInput('');
      setApiKeysInput('');
      setTestResults(null);
      setCfSaveStatus('Cleared all credentials from Windows Credential Manager.');
      setTimeout(() => setCfSaveStatus(null), 2500);
    } catch (err: any) {
      setCfValidationErr(`Clear error: ${err.toString()}`);
    }
  };

  const sections = [
    { id: 'general', label: 'General', icon: <Sliders size={14} strokeWidth={1.75} /> },
    { id: 'appearance', label: 'Appearance', icon: <Palette size={14} strokeWidth={1.75} /> },
    { id: 'api', label: 'API', icon: <Key size={14} strokeWidth={1.75} /> },
    { id: 'permissions', label: 'Permissions', icon: <Shield size={14} strokeWidth={1.75} /> },
    { id: 'memory', label: 'Memory', icon: <Brain size={14} strokeWidth={1.75} /> },
    { id: 'shortcuts', label: 'Shortcuts', icon: <Keyboard size={14} strokeWidth={1.75} /> },
  ] as const;

  const shortcutsList: ShortcutItem[] = [
    {
      id: 'search',
      label: 'Search / Jump To',
      category: 'general',
      keys: ['Ctrl', 'K'],
      description: 'Open quick search across tasks, navigation and actions.',
    },
    {
      id: 'toggle-sidebar',
      label: 'Toggle Sidebar',
      category: 'navigation',
      keys: ['Ctrl', 'B'],
      description: 'Collapse or expand navigation sidebar.',
    },
    {
      id: 'submit-task',
      label: 'Submit Prompt / Task',
      category: 'task',
      keys: ['Enter'],
      description: 'Submit prompt from the central task input field.',
    },
    {
      id: 'nav-new-chat',
      label: 'New Chat',
      category: 'navigation',
      keys: ['Ctrl', '1'],
      description: 'Reset to New Chat prompt view.',
    },
    {
      id: 'nav-activity',
      label: 'Activity',
      category: 'navigation',
      keys: ['Ctrl', '2'],
      description: 'Switch to chronological activity log.',
    },
    {
      id: 'nav-plugins',
      label: 'Plugins',
      category: 'navigation',
      keys: ['Ctrl', '3'],
      description: 'Open MCP & external plugins configuration.',
    },
    {
      id: 'open-settings',
      label: 'Settings',
      category: 'navigation',
      keys: ['Ctrl', ','],
      description: 'Open application preferences view.',
    },
  ];

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        backgroundColor: 'rgba(10, 11, 14, 0.72)',
        backdropFilter: 'blur(4px)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        zIndex: 1000,
        userSelect: 'none',
      }}
      onClick={(e) => {
        if (e.target === e.currentTarget) {
          onClose();
        }
      }}
    >
      {/* Centered Modal Window */}
      <div
        style={{
          width: '760px',
          maxWidth: '92vw',
          height: '560px',
          maxHeight: '86vh',
          backgroundColor: 'var(--bg-panel)',
          border: '1px solid var(--border-subtle)',
          borderRadius: 'var(--radius-xl)',
          boxShadow: '0 24px 64px rgba(0, 0, 0, 0.7), 0 0 0 1px var(--border-subtle)',
          display: 'flex',
          flexDirection: 'column',
          overflow: 'hidden',
        }}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Modal Header */}
        <div
          style={{
            height: '42px',
            borderBottom: '1px solid var(--border-subtle)',
            padding: '0 16px',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            flexShrink: 0,
            backgroundColor: 'var(--bg-panel)',
          }}
        >
          <span style={{ fontSize: '13px', fontWeight: 600, color: 'var(--text-primary)' }}>
            Settings
          </span>

          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <Button
              variant="primary"
              size="sm"
              onClick={handleSaveAll}
              icon={savedSuccess ? <Check size={12} /> : <Save size={12} />}
            >
              {savedSuccess ? 'Saved' : 'Save'}
            </Button>

            <button
              onClick={onClose}
              style={{
                width: '26px',
                height: '26px',
                borderRadius: 'var(--radius-sm)',
                color: 'var(--text-muted)',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                cursor: 'pointer',
                backgroundColor: 'transparent',
                border: 'none',
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.color = 'var(--text-primary)';
                e.currentTarget.style.backgroundColor = 'var(--bg-input)';
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.color = 'var(--text-muted)';
                e.currentTarget.style.backgroundColor = 'transparent';
              }}
              title="Close Settings (Esc)"
            >
              <X size={15} />
            </button>
          </div>
        </div>

        {/* Modal Body: Left Navigation + Right Content */}
        <div style={{ flex: 1, display: 'flex', overflow: 'hidden' }}>
          {/* Left Sub-Navigation */}
          <div
            style={{
              width: '180px',
              borderRight: '1px solid var(--border-subtle)',
              backgroundColor: 'var(--bg-sidebar)',
              padding: '12px 6px',
              display: 'flex',
              flexDirection: 'column',
              gap: '2px',
              flexShrink: 0,
            }}
          >
            {sections.map((sec) => {
              const isActive = activeSection === sec.id;
              return (
                <button
                  key={sec.id}
                  onClick={() => setActiveSection(sec.id)}
                  style={{
                    width: '100%',
                    padding: '7px 10px',
                    borderRadius: 'var(--radius-md)',
                    backgroundColor: isActive ? 'var(--bg-panel)' : 'transparent',
                    color: isActive ? 'var(--text-primary)' : 'var(--text-secondary)',
                    border: isActive ? '1px solid var(--border-subtle)' : '1px solid transparent',
                    fontSize: '13px',
                    fontWeight: isActive ? 500 : 400,
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'flex-start',
                    gap: '8px',
                    cursor: 'pointer',
                    textAlign: 'left',
                  }}
                >
                  <span style={{ color: isActive ? 'var(--accent-primary)' : 'inherit', display: 'flex' }}>
                    {sec.icon}
                  </span>
                  <span>{sec.label}</span>
                </button>
              );
            })}
          </div>

          {/* Right Scrollable Content Area */}
          <div
            style={{
              flex: 1,
              overflowY: 'auto',
              padding: '20px 24px',
              backgroundColor: 'var(--bg-panel)',
              textAlign: 'left',
            }}
          >
            <div style={{ maxWidth: '520px', textAlign: 'left' }}>
              {/* 1. General */}
              {activeSection === 'general' && (
                <div style={{ textAlign: 'left' }}>
                  <h2 style={{ fontSize: '14px', fontWeight: 600, color: 'var(--text-primary)', marginBottom: '3px' }}>
                    General
                  </h2>
                  <p style={{ fontSize: '12px', color: 'var(--text-secondary)', marginBottom: '18px' }}>
                    Workspace directory and startup preferences
                  </p>

                  <div style={{ display: 'flex', flexDirection: 'column', gap: '14px' }}>
                    <div>
                      <label style={{ fontSize: '12px', fontWeight: 500, color: 'var(--text-primary)', display: 'block', marginBottom: '6px' }}>
                        Default Workspace Directory
                      </label>
                      <div style={{ display: 'flex', gap: '8px' }}>
                        <input
                          type="text"
                          value={form.general.defaultWorkspace}
                          onChange={(e) =>
                            setForm({
                              ...form,
                              general: { ...form.general, defaultWorkspace: e.target.value },
                            })
                          }
                          style={{
                            flex: 1,
                            height: '32px',
                            padding: '0 10px',
                            backgroundColor: 'var(--bg-input)',
                            border: '1px solid var(--border-subtle)',
                            borderRadius: 'var(--radius-md)',
                            color: 'var(--text-primary)',
                            caretColor: 'var(--accent-primary)',
                            cursor: 'text',
                            fontFamily: 'var(--font-mono)',
                            fontSize: '12px',
                          }}
                        />
                        <Button variant="secondary" size="sm" icon={<Folder size={13} />}>
                          Browse
                        </Button>
                      </div>
                    </div>

                    <div style={{ height: '1px', backgroundColor: 'var(--border-subtle)' }} />

                    <Switch
                      label="Run in Background on Close"
                      description="Keep Aven running in system tray when main window is closed."
                      checked={form.general.runInBackground}
                      onChange={(checked) =>
                        setForm({
                          ...form,
                          general: { ...form.general, runInBackground: checked },
                        })
                      }
                    />

                    <div style={{ height: '1px', backgroundColor: 'var(--border-subtle)' }} />

                    <Switch
                      label="Launch on Windows Startup"
                      description="Automatically start Aven when logging into Windows."
                      checked={form.general.launchAtStartup}
                      onChange={(checked) =>
                        setForm({
                          ...form,
                          general: { ...form.general, launchAtStartup: checked },
                        })
                      }
                    />

                    <div style={{ height: '1px', backgroundColor: 'var(--border-subtle)' }} />

                    <Switch
                      label="Confirm Dangerous Operations"
                      description="Require user confirmation before deleting files or modifying branches."
                      checked={form.general.confirmDangerousCommands}
                      onChange={(checked) =>
                        setForm({
                          ...form,
                          general: { ...form.general, confirmDangerousCommands: checked },
                        })
                      }
                    />

                    <div style={{ height: '1px', backgroundColor: 'var(--border-subtle)' }} />

                    <Switch
                      label="Anonymous Telemetry"
                      description="Send anonymized reports to improve Aven stability."
                      checked={form.general.telemetryEnabled}
                      onChange={(checked) =>
                        setForm({
                          ...form,
                          general: { ...form.general, telemetryEnabled: checked },
                        })
                      }
                    />
                  </div>
                </div>
              )}

              {/* 2. Appearance */}
              {activeSection === 'appearance' && (
                <div>
                  <h2 style={{ fontSize: '14px', fontWeight: 600, color: 'var(--text-primary)', marginBottom: '3px' }}>
                    Appearance
                  </h2>
                  <p style={{ fontSize: '12px', color: 'var(--text-secondary)', marginBottom: '18px' }}>
                    Linear-inspired dark interface theme and layout density
                  </p>

                  <div style={{ display: 'flex', flexDirection: 'column', gap: '14px' }}>
                    <div>
                      <label style={{ fontSize: '12px', fontWeight: 500, color: 'var(--text-primary)', display: 'block', marginBottom: '8px' }}>
                        Dark Palette Theme
                      </label>
                      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: '8px' }}>
                        {[
                          { id: 'dark-linear', name: 'Linear Dark', bg: '#191A1F' },
                          { id: 'dark-midnight', name: 'Midnight', bg: '#0F1014' },
                          { id: 'dark-slate', name: 'Slate Dark', bg: '#161922' },
                        ].map((th) => {
                          const isSelected = form.appearance.theme === th.id;
                          return (
                            <button
                              key={th.id}
                              type="button"
                              onClick={() =>
                                setForm({
                                  ...form,
                                  appearance: { ...form.appearance, theme: th.id as any },
                                })
                              }
                              style={{
                                padding: '8px 10px',
                                borderRadius: 'var(--radius-md)',
                                backgroundColor: th.bg,
                                border: `1px solid ${isSelected ? 'var(--accent-primary)' : 'var(--border-subtle)'}`,
                                color: 'var(--text-primary)',
                                fontSize: '12px',
                                fontWeight: isSelected ? 500 : 400,
                                cursor: 'pointer',
                                display: 'flex',
                                alignItems: 'center',
                                justifyContent: 'space-between',
                              }}
                            >
                              <span>{th.name}</span>
                              {isSelected && <Check size={12} color="var(--accent-primary)" />}
                            </button>
                          );
                        })}
                      </div>
                    </div>

                    <div style={{ height: '1px', backgroundColor: 'var(--border-subtle)' }} />

                    <Switch
                      label="Compact Density"
                      description="Reduce padding and row heights for higher information density."
                      checked={form.appearance.compactDensity}
                      onChange={(checked) =>
                        setForm({
                          ...form,
                          appearance: { ...form.appearance, compactDensity: checked },
                        })
                      }
                    />

                    <div style={{ height: '1px', backgroundColor: 'var(--border-subtle)' }} />

                    <Switch
                      label="Status Bar"
                      description="Show bottom telemetry and status indicator bar."
                      checked={form.appearance.showStatusBar}
                      onChange={(checked) =>
                        setForm({
                          ...form,
                          appearance: { ...form.appearance, showStatusBar: checked },
                        })
                      }
                    />
                  </div>
                </div>
              )}

              {/* 3. API (Cloudflare Workers AI Exclusive Configuration) */}
              {activeSection === 'api' && (
                <div>
                  <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '3px' }}>
                    <h2 style={{ fontSize: '14px', fontWeight: 600, color: 'var(--text-primary)' }}>
                      Cloudflare Workers AI Configuration
                    </h2>
                    <span
                      style={{
                        fontSize: '11px',
                        padding: '2px 8px',
                        borderRadius: 'var(--radius-sm)',
                        backgroundColor: 'var(--accent-subtle)',
                        border: '1px solid var(--accent-border)',
                        color: 'var(--accent-primary)',
                        fontFamily: 'var(--font-mono)',
                      }}
                    >
                      @cf/qwen/qwen3.8-27b
                    </span>
                  </div>
                  <p style={{ fontSize: '12px', color: 'var(--text-secondary)', marginBottom: '16px' }}>
                    Configure multiple Cloudflare Account IDs and API Keys with automatic failover and OS credential isolation.
                  </p>

                  <div style={{ display: 'flex', flexDirection: 'column', gap: '14px' }}>
                    {/* Model Details Card */}
                    <div
                      style={{
                        padding: '12px 14px',
                        borderRadius: 'var(--radius-md)',
                        backgroundColor: 'var(--bg-panel)',
                        border: '1px solid var(--border-subtle)',
                        display: 'flex',
                        flexDirection: 'column',
                        gap: '6px',
                      }}
                    >
                      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                        <span style={{ fontSize: '12px', fontWeight: 600, color: 'var(--text-primary)' }}>
                          Model Information
                        </span>
                        <span style={{ fontSize: '11px', color: 'var(--text-muted)' }}>
                          Cloudflare Workers AI Exclusive
                        </span>
                      </div>
                      <div style={{ fontSize: '11.5px', color: 'var(--text-secondary)', lineHeight: 1.5 }}>
                        <p style={{ margin: '0 0 4px 0' }}>
                          <strong style={{ color: 'var(--text-primary)' }}>Model ID:</strong>{' '}
                          <code style={{ fontFamily: 'var(--font-mono)', color: 'var(--accent-primary)', backgroundColor: 'var(--bg-input)', padding: '1px 5px', borderRadius: '3px' }}>
                            @cf/qwen/qwen3.8-27b
                          </code>
                        </p>
                        <p style={{ margin: '0 0 4px 0' }}>
                          <strong style={{ color: 'var(--text-primary)' }}>Endpoint:</strong>{' '}
                          <code style={{ fontFamily: 'var(--font-mono)', fontSize: '11px', color: 'var(--text-muted)' }}>
                            https://api.cloudflare.com/client/v4/accounts/&#123;ACCOUNT_ID&#125;/ai/run/@cf/qwen/qwen3.8-27b
                          </code>
                        </p>
                        <p style={{ margin: 0, color: 'var(--text-muted)' }}>
                          Supports streaming tokens via Server-Sent Events (SSE). Requests are executed directly from the native Tauri Rust backend.
                        </p>
                      </div>
                    </div>

                    {/* How Credential Pairing Works Notice */}
                    <div
                      style={{
                        padding: '10px 12px',
                        borderRadius: 'var(--radius-md)',
                        backgroundColor: '#1E2028',
                        border: '1px solid #323544',
                        display: 'flex',
                        gap: '10px',
                        alignItems: 'flex-start',
                      }}
                    >
                      <Info size={16} color="var(--accent-primary)" style={{ flexShrink: 0, marginTop: '2px' }} />
                      <div style={{ fontSize: '11.5px', color: 'var(--text-secondary)', lineHeight: 1.45 }}>
                        <strong style={{ color: 'var(--text-primary)', display: 'block', marginBottom: '2px' }}>
                          How Multiple Credentials and Failover Work:
                        </strong>
                        Enter comma-separated values for Account IDs and API Keys. They are paired sequentially in order:
                        <ul style={{ margin: '4px 0 0 16px', padding: 0 }}>
                          <li><strong>Pair 1:</strong> Account 1 + API Key 1 (preferred primary)</li>
                          <li><strong>Pair 2:</strong> Account 2 + API Key 2 (automatic fallback on rate limits or errors)</li>
                        </ul>
                        Whitespace is trimmed. Leading, trailing, and repeated commas are ignored. All keys are encrypted in the native Windows Credential Manager.
                      </div>
                    </div>

                    {/* Account IDs Input */}
                    <div>
                      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '6px' }}>
                        <label style={{ fontSize: '12px', fontWeight: 500, color: 'var(--text-primary)' }}>
                          Cloudflare Account IDs (comma-separated)
                        </label>
                        {parsedAccountIds.length > 0 && (
                          <span style={{ fontSize: '11px', color: 'var(--accent-primary)' }}>
                            {parsedAccountIds.length} account{parsedAccountIds.length > 1 ? 's' : ''} detected
                          </span>
                        )}
                      </div>
                      <input
                        type="text"
                        value={accountIdsInput}
                        onChange={(e) => setAccountIdsInput(e.target.value)}
                        placeholder="e.g. account_id_1, account_id_2, account_id_3"
                        style={{
                          width: '100%',
                          height: '32px',
                          padding: '0 10px',
                          backgroundColor: 'var(--bg-input)',
                          border: '1px solid var(--border-subtle)',
                          borderRadius: 'var(--radius-md)',
                          color: 'var(--text-primary)',
                          caretColor: 'var(--accent-primary)',
                          fontSize: '12.5px',
                          fontFamily: 'var(--font-mono)',
                        }}
                      />
                    </div>

                    {/* API Keys Input */}
                    <div>
                      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '6px' }}>
                        <label style={{ fontSize: '12px', fontWeight: 500, color: 'var(--text-primary)' }}>
                          Cloudflare API Keys (comma-separated)
                        </label>
                        {parsedApiKeys.length > 0 && (
                          <span style={{ fontSize: '11px', color: 'var(--accent-primary)' }}>
                            {parsedApiKeys.length} key{parsedApiKeys.length > 1 ? 's' : ''} detected
                          </span>
                        )}
                      </div>
                      <div style={{ position: 'relative', display: 'flex', alignItems: 'center' }}>
                        <input
                          type={showApiKeys ? 'text' : 'password'}
                          value={apiKeysInput}
                          onChange={(e) => setApiKeysInput(e.target.value)}
                          placeholder="e.g. api_key_1, api_key_2, api_key_3"
                          style={{
                            width: '100%',
                            height: '32px',
                            padding: '0 34px 0 10px',
                            backgroundColor: 'var(--bg-input)',
                            border: '1px solid var(--border-subtle)',
                            borderRadius: 'var(--radius-md)',
                            color: 'var(--text-primary)',
                            caretColor: 'var(--accent-primary)',
                            fontSize: '12.5px',
                            fontFamily: showApiKeys ? 'var(--font-mono)' : 'inherit',
                          }}
                        />
                        <button
                          type="button"
                          onClick={() => setShowApiKeys(!showApiKeys)}
                          style={{
                            position: 'absolute',
                            right: '8px',
                            background: 'transparent',
                            border: 'none',
                            color: 'var(--text-muted)',
                            cursor: 'pointer',
                            display: 'flex',
                            alignItems: 'center',
                          }}
                          title={showApiKeys ? 'Hide API keys' : 'Show API keys'}
                        >
                          {showApiKeys ? <EyeOff size={14} /> : <Eye size={14} />}
                        </button>
                      </div>
                    </div>

                    {/* Pairing & Count Validation Indicator */}
                    {accountIdsInput.trim() && apiKeysInput.trim() && (
                      <div
                        style={{
                          padding: '6px 10px',
                          borderRadius: 'var(--radius-sm)',
                          backgroundColor: parsedAccountIds.length === parsedApiKeys.length ? 'rgba(16, 185, 129, 0.1)' : 'rgba(239, 68, 68, 0.1)',
                          border: `1px solid ${parsedAccountIds.length === parsedApiKeys.length ? 'rgba(16, 185, 129, 0.25)' : 'rgba(239, 68, 68, 0.25)'}`,
                          display: 'flex',
                          alignItems: 'center',
                          gap: '6px',
                          fontSize: '11px',
                          color: parsedAccountIds.length === parsedApiKeys.length ? '#34D399' : 'var(--status-danger)',
                        }}
                      >
                        {parsedAccountIds.length === parsedApiKeys.length ? (
                          <>
                            <Check size={13} />
                            <span>
                              Valid pairing: {parsedAccountIds.length} Account ID{parsedAccountIds.length > 1 ? 's' : ''} matched 1-to-1 with {parsedApiKeys.length} API Key{parsedApiKeys.length > 1 ? 's' : ''}.
                            </span>
                          </>
                        ) : (
                          <>
                            <AlertTriangle size={13} />
                            <span>
                              Mismatched count: {parsedAccountIds.length} Account ID(s) vs {parsedApiKeys.length} API Key(s). Each Account ID must have a matching API Key.
                            </span>
                          </>
                        )}
                      </div>
                    )}

                    {/* Save and Clear Action Buttons */}
                    <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                      <Button
                        variant="primary"
                        size="sm"
                        onClick={handleSaveCloudflare}
                        disabled={isSavingCf || !accountIdsInput.trim() || !apiKeysInput.trim()}
                        icon={isSavingCf ? <RefreshCw size={13} className="animate-spin" /> : <Save size={13} />}
                      >
                        {isSavingCf ? 'Saving to OS Credential Manager...' : 'Save Credentials'}
                      </Button>

                      {cfStatus && cfStatus.configured_count > 0 && (
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={handleClearCloudflare}
                          title="Remove all credentials from Windows Credential Manager"
                        >
                          <Trash2 size={13} color="var(--status-danger)" />
                          <span style={{ color: 'var(--status-danger)' }}>Clear Stored Credentials</span>
                        </Button>
                      )}
                    </div>

                    {/* Feedback Messages */}
                    {cfValidationErr && (
                      <div
                        style={{
                          padding: '8px 10px',
                          borderRadius: 'var(--radius-sm)',
                          backgroundColor: 'rgba(239, 68, 68, 0.1)',
                          border: '1px solid rgba(239, 68, 68, 0.3)',
                          color: 'var(--status-danger)',
                          fontSize: '11.5px',
                          display: 'flex',
                          alignItems: 'center',
                          gap: '6px',
                        }}
                      >
                        <AlertCircle size={14} style={{ flexShrink: 0 }} />
                        <span>{cfValidationErr}</span>
                      </div>
                    )}

                    {cfSaveStatus && (
                      <div
                        style={{
                          padding: '8px 10px',
                          borderRadius: 'var(--radius-sm)',
                          backgroundColor: 'rgba(16, 185, 129, 0.1)',
                          border: '1px solid rgba(16, 185, 129, 0.3)',
                          color: '#34D399',
                          fontSize: '11.5px',
                          display: 'flex',
                          alignItems: 'center',
                          gap: '6px',
                        }}
                      >
                        <CheckCircle2 size={14} style={{ flexShrink: 0 }} />
                        <span>{cfSaveStatus}</span>
                      </div>
                    )}

                    <div style={{ height: '1px', backgroundColor: 'var(--border-subtle)' }} />

                    {/* Configured Credentials Status & Test Connection Section */}
                    <div>
                      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '8px' }}>
                        <div>
                          <label style={{ fontSize: '12px', fontWeight: 600, color: 'var(--text-primary)', display: 'block' }}>
                            OS Secure Credential Storage Status
                          </label>
                          <span style={{ fontSize: '11px', color: 'var(--text-muted)' }}>
                            Encrypted in Windows Credential Manager (never exposed to frontend state)
                          </span>
                        </div>

                        <Button
                          variant="secondary"
                          size="sm"
                          onClick={handleTestConnection}
                          disabled={isTestingCf || !cfStatus || cfStatus.configured_count === 0}
                          icon={isTestingCf ? <RefreshCw size={12} className="animate-spin" /> : <RefreshCw size={12} />}
                        >
                          {isTestingCf ? 'Testing All Pairs...' : 'Test Connection'}
                        </Button>
                      </div>

                      {/* Configured Pairs List or Empty State */}
                      {cfStatus && cfStatus.configured_count > 0 ? (
                        <div
                          style={{
                            display: 'flex',
                            flexDirection: 'column',
                            gap: '6px',
                            padding: '10px 12px',
                            borderRadius: 'var(--radius-md)',
                            backgroundColor: 'var(--bg-panel)',
                            border: '1px solid var(--border-subtle)',
                          }}
                        >
                          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '4px' }}>
                            <span style={{ fontSize: '11.5px', fontWeight: 500, color: 'var(--text-primary)' }}>
                              Configured Credential Pairs ({cfStatus.configured_count}):
                            </span>
                            <span style={{ fontSize: '11px', color: 'var(--accent-primary)', fontFamily: 'var(--font-mono)' }}>
                              Preferred: Pair #{cfStatus.preferred_index + 1}
                            </span>
                          </div>

                          {cfStatus.pairs.map((p) => {
                            const isPreferred = p.pair_index === cfStatus.preferred_index;
                            const pairTest = testResults?.results.find((r) => r.pair_index === p.pair_index);

                            return (
                              <div
                                key={p.pair_index}
                                style={{
                                  display: 'flex',
                                  alignItems: 'center',
                                  justifyContent: 'space-between',
                                  padding: '6px 10px',
                                  borderRadius: 'var(--radius-sm)',
                                  backgroundColor: 'var(--bg-main)',
                                  border: `1px solid ${isPreferred ? 'var(--accent-border)' : 'var(--border-subtle)'}`,
                                  fontSize: '11.5px',
                                }}
                              >
                                <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                                  <span style={{ fontWeight: 600, color: 'var(--text-primary)', fontFamily: 'var(--font-mono)' }}>
                                    Pair #{p.pair_index + 1}
                                  </span>
                                  <span style={{ color: 'var(--text-secondary)', fontFamily: 'var(--font-mono)' }}>
                                    Account: {p.account_id_masked}
                                  </span>
                                  <span style={{ color: 'var(--text-muted)' }}>
                                    • Key: ••••••••
                                  </span>
                                  {isPreferred && (
                                    <span
                                      style={{
                                        fontSize: '10px',
                                        padding: '1px 5px',
                                        borderRadius: '3px',
                                        backgroundColor: 'var(--accent-subtle)',
                                        color: 'var(--accent-primary)',
                                        fontWeight: 500,
                                      }}
                                    >
                                      Primary
                                    </span>
                                  )}
                                </div>

                                {/* Test Status for this pair */}
                                <div>
                                  {pairTest ? (
                                    pairTest.status === 'connected' ? (
                                      <span
                                        style={{
                                          fontSize: '11px',
                                          color: '#34D399',
                                          display: 'inline-flex',
                                          alignItems: 'center',
                                          gap: '4px',
                                          fontWeight: 500,
                                        }}
                                      >
                                        <CheckCircle2 size={12} /> Connected
                                      </span>
                                    ) : (
                                      <span
                                        style={{
                                          fontSize: '11px',
                                          color: 'var(--status-danger)',
                                          display: 'inline-flex',
                                          alignItems: 'center',
                                          gap: '4px',
                                          fontWeight: 500,
                                        }}
                                        title={pairTest.error_message || 'Verification failed'}
                                      >
                                        <AlertCircle size={12} /> Failed
                                      </span>
                                    )
                                  ) : (
                                    <span style={{ fontSize: '11px', color: 'var(--text-muted)' }}>
                                      Not tested
                                    </span>
                                  )}
                                </div>
                              </div>
                            );
                          })}

                          {/* Overall Test Status Banner */}
                          {testResults && (
                            <div
                              style={{
                                marginTop: '6px',
                                padding: '8px 10px',
                                borderRadius: 'var(--radius-sm)',
                                backgroundColor: testResults.any_connected ? 'rgba(16, 185, 129, 0.1)' : 'rgba(239, 68, 68, 0.1)',
                                border: `1px solid ${testResults.any_connected ? 'rgba(16, 185, 129, 0.3)' : 'rgba(239, 68, 68, 0.3)'}`,
                                fontSize: '11.5px',
                              }}
                            >
                              <div style={{ display: 'flex', alignItems: 'center', gap: '6px', fontWeight: 600, color: testResults.any_connected ? '#34D399' : 'var(--status-danger)' }}>
                                {testResults.any_connected ? <CheckCircle2 size={14} /> : <AlertCircle size={14} />}
                                <span>
                                  {testResults.any_connected
                                    ? 'Overall Status: Cloudflare Workers AI is ready. At least one credential pair verified.'
                                    : 'Overall Status: All credential pairs failed verification.'}
                                </span>
                              </div>
                              {testResults.results.some((r) => r.status === 'failed') && (
                                <div style={{ marginTop: '4px', fontSize: '11px', color: 'var(--text-secondary)' }}>
                                  {testResults.results
                                    .filter((r) => r.status === 'failed')
                                    .map((r) => (
                                      <div key={r.pair_index}>
                                        • Pair #{r.pair_index + 1} ({r.account_id_masked}): {r.error_message}
                                      </div>
                                    ))}
                                </div>
                              )}
                            </div>
                          )}
                        </div>
                      ) : (
                        <div
                          style={{
                            padding: '16px',
                            textAlign: 'center',
                            borderRadius: 'var(--radius-md)',
                            backgroundColor: 'var(--bg-panel)',
                            border: '1px dashed var(--border-subtle)',
                            color: 'var(--text-muted)',
                            fontSize: '12px',
                          }}
                        >
                          No Cloudflare credential pairs currently configured. Enter comma-separated Account IDs and API Keys above and click "Save Credentials".
                        </div>
                      )}
                    </div>

                    <div style={{ height: '1px', backgroundColor: 'var(--border-subtle)' }} />

                    {/* Temperature Setting */}
                    <div>
                      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '6px' }}>
                        <label style={{ fontSize: '12px', fontWeight: 500, color: 'var(--text-primary)' }}>
                          Temperature
                        </label>
                        <span style={{ fontSize: '12px', color: 'var(--accent-primary)', fontFamily: 'var(--font-mono)' }}>
                          {form.api.temperature.toFixed(2)}
                        </span>
                      </div>
                      <input
                        type="range"
                        min="0"
                        max="1"
                        step="0.05"
                        value={form.api.temperature}
                        onChange={(e) =>
                          setForm({
                            ...form,
                            api: { ...form.api, temperature: parseFloat(e.target.value) },
                          })
                        }
                        style={{ width: '100%', accentColor: 'var(--accent-primary)' }}
                      />
                    </div>
                  </div>
                </div>
              )}

              {/* 4. Permissions */}
              {activeSection === 'permissions' && (
                <div>
                  <h2 style={{ fontSize: '14px', fontWeight: 600, color: 'var(--text-primary)', marginBottom: '3px' }}>
                    Permissions
                  </h2>
                  <p style={{ fontSize: '12px', color: 'var(--text-secondary)', marginBottom: '18px' }}>
                    Sandbox execution and workspace access policies
                  </p>

                  <div style={{ display: 'flex', flexDirection: 'column', gap: '14px' }}>
                    <div>
                      <label style={{ fontSize: '12px', fontWeight: 500, color: 'var(--text-primary)', display: 'block', marginBottom: '6px' }}>
                        Workspace File Access
                      </label>
                      <select
                        value={form.permissions.fileSystemAccess}
                        onChange={(e) =>
                          setForm({
                            ...form,
                            permissions: { ...form.permissions, fileSystemAccess: e.target.value as any },
                          })
                        }
                        style={{
                          width: '100%',
                          height: '32px',
                          padding: '0 10px',
                          backgroundColor: 'var(--bg-input)',
                          border: '1px solid var(--border-subtle)',
                          borderRadius: 'var(--radius-md)',
                          color: 'var(--text-primary)',
                          fontSize: '13px',
                        }}
                      >
                        <option value="read-write">Read & Write (Full workspace editing)</option>
                        <option value="prompt-each">Prompt before modifying files</option>
                        <option value="read-only">Read Only (Inspection only)</option>
                      </select>
                    </div>

                    <div style={{ height: '1px', backgroundColor: 'var(--border-subtle)' }} />

                    <div>
                      <label style={{ fontSize: '12px', fontWeight: 500, color: 'var(--text-primary)', display: 'block', marginBottom: '6px' }}>
                        Shell Execution
                      </label>
                      <select
                        value={form.permissions.shellExecution}
                        onChange={(e) =>
                          setForm({
                            ...form,
                            permissions: { ...form.permissions, shellExecution: e.target.value as any },
                          })
                        }
                        style={{
                          width: '100%',
                          height: '32px',
                          padding: '0 10px',
                          backgroundColor: 'var(--bg-input)',
                          border: '1px solid var(--border-subtle)',
                          borderRadius: 'var(--radius-md)',
                          color: 'var(--text-primary)',
                          fontSize: '13px',
                        }}
                      >
                        <option value="auto-safe">Auto-approve safe commands (git, npm test)</option>
                        <option value="prompt-all">Prompt before every command</option>
                        <option value="disabled">Disabled</option>
                      </select>
                    </div>

                    <div style={{ height: '1px', backgroundColor: 'var(--border-subtle)' }} />

                    <div>
                      <label style={{ fontSize: '12px', fontWeight: 500, color: 'var(--text-primary)', display: 'block', marginBottom: '6px' }}>
                        Network Policy
                      </label>
                      <select
                        value={form.permissions.networkAccess}
                        onChange={(e) =>
                          setForm({
                            ...form,
                            permissions: { ...form.permissions, networkAccess: e.target.value as any },
                          })
                        }
                        style={{
                          width: '100%',
                          height: '32px',
                          padding: '0 10px',
                          backgroundColor: 'var(--bg-input)',
                          border: '1px solid var(--border-subtle)',
                          borderRadius: 'var(--radius-md)',
                          color: 'var(--text-primary)',
                          fontSize: '13px',
                        }}
                      >
                        <option value="full">Full Network Access</option>
                        <option value="restricted">Restricted (AI endpoints & GitHub only)</option>
                        <option value="offline">Offline / Airgapped</option>
                      </select>
                    </div>
                  </div>
                </div>
              )}

              {/* 5. Memory */}
              {activeSection === 'memory' && (
                <div>
                  <h2 style={{ fontSize: '14px', fontWeight: 600, color: 'var(--text-primary)', marginBottom: '3px' }}>
                    Memory & Context
                  </h2>
                  <p style={{ fontSize: '12px', color: 'var(--text-secondary)', marginBottom: '18px' }}>
                    Knowledge retention and context window limits
                  </p>

                  <div style={{ display: 'flex', flexDirection: 'column', gap: '14px' }}>
                    <Switch
                      label="Persistent Agent Memory"
                      description="Retain context across task sessions and project workspaces."
                      checked={form.memory.enableMemory}
                      onChange={(checked) =>
                        setForm({
                          ...form,
                          memory: { ...form.memory, enableMemory: checked },
                        })
                      }
                    />

                    <div style={{ height: '1px', backgroundColor: 'var(--border-subtle)' }} />

                    <Switch
                      label="Local Vector Index"
                      description="Index codebase AST locally for fast semantic retrieval."
                      checked={form.memory.localVectorIndex}
                      onChange={(checked) =>
                        setForm({
                          ...form,
                          memory: { ...form.memory, localVectorIndex: checked },
                        })
                      }
                    />

                    <div style={{ height: '1px', backgroundColor: 'var(--border-subtle)' }} />

                    <div>
                      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '6px' }}>
                        <label style={{ fontSize: '12px', fontWeight: 500, color: 'var(--text-primary)' }}>
                          Max Context Tokens
                        </label>
                        <span style={{ fontSize: '12px', color: 'var(--accent-primary)', fontFamily: 'var(--font-mono)' }}>
                          {form.memory.maxContextTokens.toLocaleString()} tokens
                        </span>
                      </div>
                      <input
                        type="range"
                        min="8000"
                        max="128000"
                        step="8000"
                        value={form.memory.maxContextTokens}
                        onChange={(e) =>
                          setForm({
                            ...form,
                            memory: { ...form.memory, maxContextTokens: Number(e.target.value) },
                          })
                        }
                        style={{ width: '100%', accentColor: 'var(--accent-primary)' }}
                      />
                    </div>
                  </div>
                </div>
              )}

              {/* 6. Shortcuts */}
              {activeSection === 'shortcuts' && (
                <div>
                  <h2 style={{ fontSize: '14px', fontWeight: 600, color: 'var(--text-primary)', marginBottom: '3px' }}>
                    Shortcuts
                  </h2>
                  <p style={{ fontSize: '12px', color: 'var(--text-secondary)', marginBottom: '18px' }}>
                    Keyboard shortcuts for fast navigation and actions
                  </p>

                  <div style={{ display: 'flex', flexDirection: 'column', gap: '1px' }}>
                    {shortcutsList.map((item, idx) => (
                      <div
                        key={item.id}
                        style={{
                          display: 'flex',
                          alignItems: 'center',
                          justifyContent: 'space-between',
                          padding: '8px 0',
                          borderBottom:
                            idx === shortcutsList.length - 1 ? 'none' : '1px solid var(--border-subtle)',
                        }}
                      >
                        <div>
                          <div style={{ fontSize: '13px', fontWeight: 500, color: 'var(--text-primary)' }}>
                            {item.label}
                          </div>
                          <div style={{ fontSize: '11px', color: 'var(--text-muted)', marginTop: '1px' }}>
                            {item.description}
                          </div>
                        </div>

                        <div style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
                          {item.keys.map((k, ki) => (
                            <kbd
                              key={ki}
                              style={{
                                padding: '2px 6px',
                                backgroundColor: 'var(--bg-input)',
                                border: '1px solid var(--border-subtle)',
                                borderRadius: 'var(--radius-sm)',
                                fontFamily: 'var(--font-mono)',
                                fontSize: '11px',
                                color: 'var(--text-primary)',
                              }}
                            >
                              {k}
                            </kbd>
                          ))}
                        </div>
                      </div>
                    ))}
                  </div>
                </div>
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
