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
} from 'lucide-react';
import { AppSettings, ProviderId, ShortcutItem } from '../../types';
import { Button } from '../../components/common/Button';
import { Switch } from '../../components/common/Switch';
import { secureStore } from '../../services/tauriIpc';

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
  const [apiKeyInput, setApiKeyInput] = useState('');
  const [showApiKey, setShowApiKey] = useState(false);
  const [hasStoredKey, setHasStoredKey] = useState(false);
  const [apiSaveStatus, setApiSaveStatus] = useState<string | null>(null);

  useEffect(() => {
    setForm(settings);
  }, [settings]);

  useEffect(() => {
    if (isOpen) {
      secureStore.hasApiKey(form.api.activeProvider).then((exists) => {
        setHasStoredKey(exists);
        setApiKeyInput('');
        setShowApiKey(false);
        setApiSaveStatus(null);
      });
    }
  }, [form.api.activeProvider, isOpen]);

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

  const handleSaveApiKey = async () => {
    if (!apiKeyInput.trim()) return;
    try {
      setApiSaveStatus('Saving to Windows Credential Manager...');
      await secureStore.setApiKey(form.api.activeProvider, apiKeyInput.trim());
      setHasStoredKey(true);
      setApiKeyInput('');
      setShowApiKey(false);
      setApiSaveStatus('Saved in OS Credential Manager');
      setTimeout(() => setApiSaveStatus(null), 2500);
    } catch (e: any) {
      setApiSaveStatus(`Error: ${e.toString()}`);
    }
  };

  const handleRevealKey = async () => {
    if (!showApiKey) {
      try {
        const stored = await secureStore.getApiKey(form.api.activeProvider);
        if (stored) {
          setApiKeyInput(stored);
        }
      } catch {
        // fail gracefully
      }
    }
    setShowApiKey(!showApiKey);
  };

  const handleDeleteKey = async () => {
    try {
      await secureStore.deleteApiKey(form.api.activeProvider);
      setHasStoredKey(false);
      setApiKeyInput('');
      setShowApiKey(false);
      setApiSaveStatus('Key deleted from OS Credential Manager');
      setTimeout(() => setApiSaveStatus(null), 2500);
    } catch (e: any) {
      setApiSaveStatus(`Error: ${e.toString()}`);
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

              {/* 3. API (AI Provider Configuration & Keys) */}
              {activeSection === 'api' && (
                <div>
                  <h2 style={{ fontSize: '14px', fontWeight: 600, color: 'var(--text-primary)', marginBottom: '3px' }}>
                    API & Model Providers
                  </h2>
                  <p style={{ fontSize: '12px', color: 'var(--text-secondary)', marginBottom: '18px' }}>
                    Configure AI model providers and secure OS credential storage
                  </p>

                  <div style={{ display: 'flex', flexDirection: 'column', gap: '14px' }}>
                    <div>
                      <label style={{ fontSize: '12px', fontWeight: 500, color: 'var(--text-primary)', display: 'block', marginBottom: '6px' }}>
                        Model Provider
                      </label>
                      <select
                        value={form.api.activeProvider}
                        onChange={(e) =>
                          setForm({
                            ...form,
                            api: { ...form.api, activeProvider: e.target.value as ProviderId },
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
                        <option value="anthropic">Anthropic (Claude 3.7 Sonnet, Claude 3.5)</option>
                        <option value="openai">OpenAI (GPT-4o, o3-mini)</option>
                        <option value="gemini">Google Gemini (Gemini 2.5 Pro, 2.0 Flash)</option>
                        <option value="ollama">Ollama (Local Models)</option>
                        <option value="groq">Groq Cloud (Fast LPU)</option>
                        <option value="openrouter">OpenRouter (Multi-model Gateway)</option>
                      </select>
                    </div>

                    <div>
                      <label style={{ fontSize: '12px', fontWeight: 500, color: 'var(--text-primary)', display: 'block', marginBottom: '6px' }}>
                        Default Model Name
                      </label>
                      <input
                        type="text"
                        value={form.api.activeModel}
                        onChange={(e) =>
                          setForm({
                            ...form,
                            api: { ...form.api, activeModel: e.target.value },
                          })
                        }
                        placeholder="e.g. Claude 3.7 Sonnet"
                        style={{
                          width: '100%',
                          height: '32px',
                          padding: '0 10px',
                          backgroundColor: 'var(--bg-input)',
                          border: '1px solid var(--border-subtle)',
                          borderRadius: 'var(--radius-md)',
                          color: 'var(--text-primary)',
                          caretColor: 'var(--accent-primary)',
                          cursor: 'text',
                          fontSize: '13px',
                        }}
                      />
                    </div>

                    <div style={{ height: '1px', backgroundColor: 'var(--border-subtle)' }} />

                    {/* Secret Key Input with Masking & Windows Credential Manager storage */}
                    <div>
                      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '6px' }}>
                        <label style={{ fontSize: '12px', fontWeight: 500, color: 'var(--text-primary)' }}>
                          {form.api.activeProvider.toUpperCase()} API Key
                        </label>
                        <span
                          style={{
                            fontSize: '11px',
                            display: 'inline-flex',
                            alignItems: 'center',
                            gap: '4px',
                            color: hasStoredKey ? 'var(--status-success)' : 'var(--text-muted)',
                          }}
                        >
                          <span
                            style={{
                              width: '5px',
                              height: '5px',
                              borderRadius: '50%',
                              backgroundColor: hasStoredKey ? 'var(--status-success)' : 'var(--text-muted)',
                            }}
                          />
                          {hasStoredKey ? 'Stored in Windows Credential Manager' : 'No key stored'}
                        </span>
                      </div>

                      <div style={{ display: 'flex', gap: '8px' }}>
                        <div style={{ position: 'relative', flex: 1, display: 'flex', alignItems: 'center' }}>
                          <input
                            type={showApiKey ? 'text' : 'password'}
                            placeholder={hasStoredKey ? '••••••••••••••••••••••••••••' : 'Enter API Key'}
                            value={apiKeyInput}
                            onChange={(e) => setApiKeyInput(e.target.value)}
                            style={{
                              width: '100%',
                              height: '32px',
                              padding: '0 32px 0 10px',
                              backgroundColor: 'var(--bg-input)',
                              border: '1px solid var(--border-subtle)',
                              borderRadius: 'var(--radius-md)',
                              color: 'var(--text-primary)',
                              caretColor: 'var(--accent-primary)',
                              cursor: 'text',
                              fontSize: '13px',
                              fontFamily: showApiKey ? 'var(--font-mono)' : 'inherit',
                            }}
                          />
                          <button
                            type="button"
                            onClick={handleRevealKey}
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
                            title={showApiKey ? 'Hide key' : 'Show key'}
                          >
                            {showApiKey ? <EyeOff size={14} /> : <Eye size={14} />}
                          </button>
                        </div>

                        <Button
                          variant="primary"
                          size="sm"
                          onClick={handleSaveApiKey}
                          disabled={!apiKeyInput.trim()}
                        >
                          Save Key
                        </Button>

                        {hasStoredKey && (
                          <Button
                            variant="ghost"
                            size="sm"
                            onClick={handleDeleteKey}
                            title="Remove key from OS Credential Manager"
                          >
                            <Trash2 size={13} color="var(--status-danger)" />
                          </Button>
                        )}
                      </div>

                      {apiSaveStatus && (
                        <span
                          style={{
                            fontSize: '11px',
                            color: apiSaveStatus.includes('Error') ? 'var(--status-danger)' : 'var(--status-success)',
                            marginTop: '4px',
                            display: 'block',
                          }}
                        >
                          {apiSaveStatus}
                        </span>
                      )}

                      <span style={{ fontSize: '11px', color: 'var(--text-muted)', marginTop: '4px', display: 'block' }}>
                        Stored natively in the Windows Credential Manager (`keyring` crate) via Tauri Rust backend.
                      </span>
                    </div>

                    <div style={{ height: '1px', backgroundColor: 'var(--border-subtle)' }} />

                    {/* Temperature */}
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
