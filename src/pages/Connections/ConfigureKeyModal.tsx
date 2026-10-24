import React, { useState, useEffect } from 'react';
import { Eye, EyeOff, ShieldCheck, Check } from 'lucide-react';
import { ModelProvider } from '../../types';
import { Modal } from '../../components/common/Modal';
import { Button } from '../../components/common/Button';
import { Input } from '../../components/common/Input';
import { secureStore } from '../../services/tauriIpc';

export interface ConfigureKeyModalProps {
  provider: ModelProvider | null;
  isOpen: boolean;
  onClose: () => void;
  onKeySaved: (providerId: string) => void;
}

export const ConfigureKeyModal: React.FC<ConfigureKeyModalProps> = ({
  provider,
  isOpen,
  onClose,
  onKeySaved,
}) => {
  const [apiKey, setApiKey] = useState('');
  const [showKey, setShowKey] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [saveSuccess, setSaveSuccess] = useState(false);
  const [hasExistingKey, setHasExistingKey] = useState(false);
  const [errorMessage, setErrorMessage] = useState('');

  useEffect(() => {
    if (provider && isOpen) {
      setApiKey('');
      setShowKey(false);
      setSaveSuccess(false);
      setErrorMessage('');
      secureStore.hasApiKey(provider.id).then((exists) => {
        setHasExistingKey(exists);
      });
    }
  }, [provider, isOpen]);

  if (!provider) return null;

  const parsedKeys = apiKey
    .split(',')
    .map((s) => s.trim())
    .filter(Boolean);

  const handleSave = async () => {
    if (!apiKey.trim() && !hasExistingKey) {
      setErrorMessage('Please enter at least one API key');
      return;
    }

    if (apiKey.trim()) {
      // Validate no internal spaces inside individual keys
      for (let i = 0; i < parsedKeys.length; i++) {
        if (parsedKeys[i].includes(' ')) {
          setErrorMessage(`API key #${i + 1} contains invalid internal whitespace.`);
          return;
        }
      }
    }

    try {
      setIsSaving(true);
      setErrorMessage('');
      if (apiKey.trim()) {
        await secureStore.setApiKey(provider.id, apiKey.trim());
      }
      setSaveSuccess(true);
      onKeySaved(provider.id);
      setTimeout(() => {
        onClose();
      }, 1000);
    } catch (err: any) {
      setErrorMessage(err?.toString() || 'Failed to save credential');
    } finally {
      setIsSaving(false);
    }
  };

  const handleRevealKey = async () => {
    if (!showKey) {
      try {
        const stored = await secureStore.getApiKey(provider.id);
        if (stored) {
          setApiKey(stored);
        }
      } catch (err: any) {
        setErrorMessage('Failed to retrieve secret from OS credential manager');
      }
    }
    setShowKey(!showKey);
  };

  const handleDeleteKey = async () => {
    try {
      await secureStore.deleteApiKey(provider.id);
      setApiKey('');
      setHasExistingKey(false);
      onKeySaved(provider.id);
    } catch (err: any) {
      setErrorMessage('Failed to delete credential');
    }
  };

  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      title={`Configure ${provider.name}`}
      description="API keys are stored exclusively in the Windows Credential Manager via native Tauri Rust IPC."
      footer={
        <>
          {hasExistingKey && (
            <Button
              variant="danger"
              size="sm"
              onClick={handleDeleteKey}
              style={{ marginRight: 'auto' }}
            >
              Delete Key
            </Button>
          )}
          <Button variant="ghost" size="sm" onClick={onClose}>
            Cancel
          </Button>
          <Button
            variant="primary"
            size="sm"
            onClick={handleSave}
            disabled={isSaving || (!apiKey.trim() && !hasExistingKey)}
          >
            {isSaving ? 'Storing in OS Keyring...' : saveSuccess ? 'Saved!' : 'Save Key(s)'}
          </Button>
        </>
      }
    >
      <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-4)' }}>
        {/* Security Assurance Banner */}
        <div
          style={{
            display: 'flex',
            alignItems: 'flex-start',
            gap: '10px',
            padding: '10px 12px',
            backgroundColor: 'var(--bg-main)',
            border: '1px solid var(--border-subtle)',
            borderRadius: 'var(--radius-md)',
          }}
        >
          <ShieldCheck size={16} color="var(--status-success)" style={{ flexShrink: 0, marginTop: '2px' }} />
          <div style={{ fontSize: '11px', color: 'var(--text-secondary)', lineHeight: 1.45 }}>
            <span style={{ fontWeight: 600, color: 'var(--text-primary)' }}>Secure OS Isolation: </span>
            Aven never writes credentials to plaintext local storage or configuration files. Secrets are committed to the Windows Credential Manager and only decrypted on demand. Multiple keys are supported separated by commas for automatic rate-limit failover.
          </div>
        </div>

        {hasExistingKey && (
          <div
            style={{
              padding: '8px 12px',
              backgroundColor: 'var(--status-success-bg)',
              border: '1px solid var(--status-success-border)',
              borderRadius: 'var(--radius-md)',
              fontSize: '12px',
              color: 'var(--status-success)',
              display: 'flex',
              alignItems: 'center',
              gap: '6px',
            }}
          >
            <Check size={14} />
            <span>Active API Key(s) detected in Windows Credential Manager</span>
          </div>
        )}

        {/* Input */}
        <Input
          label={`${provider.name} API Key(s) (comma-separated for auto-failover)`}
          type={showKey ? 'text' : 'password'}
          placeholder={hasExistingKey ? '••••••••••••••••••••••••••••' : `Enter ${provider.name} API Key(s)`}
          value={apiKey}
          onChange={(e) => setApiKey(e.target.value)}
          errorText={errorMessage}
          helperText={
            parsedKeys.length > 1
              ? `${parsedKeys.length} keys detected for automatic rate-limit failover.`
              : `Enter multiple keys separated by commas for automatic failover. Available for models like: ${provider.availableModels.slice(0, 3).join(', ')}`
          }
          suffixAction={
            <button
              type="button"
              onClick={handleRevealKey}
              style={{
                background: 'transparent',
                border: 'none',
                color: 'var(--text-muted)',
                cursor: 'pointer',
                padding: '4px',
              }}
              title={showKey ? 'Hide key' : 'Show key'}
            >
              {showKey ? <EyeOff size={14} /> : <Eye size={14} />}
            </button>
          }
        />

        {/* Default Model info */}
        <div
          style={{
            fontSize: '12px',
            color: 'var(--text-muted)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            paddingTop: '6px',
          }}
        >
          <span>Default model: {provider.defaultModel}</span>
          <a
            href={provider.docsUrl}
            target="_blank"
            rel="noreferrer"
            style={{ color: 'var(--accent-primary)', textDecoration: 'none', fontSize: '11px' }}
          >
            Get API key →
          </a>
        </div>
      </div>
    </Modal>
  );
};
