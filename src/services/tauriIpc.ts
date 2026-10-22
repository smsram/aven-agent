import { invoke } from '@tauri-apps/api/core';
import { AppSettings, ProviderId } from '../types';

// Check if running inside Tauri desktop environment
export const isTauriDesktop = (): boolean => {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
};

// In-memory fallback for browser dev previews only (never persisted to disk or localStorage)
const devMemoryKeyStore: Record<string, string> = {
  anthropic: '',
  openai: '',
  gemini: '',
  groq: '',
};

let devSettingsMemory: AppSettings | null = null;

export const secureStore = {
  async setApiKey(provider: ProviderId | string, key: string): Promise<void> {
    if (isTauriDesktop()) {
      await invoke('set_api_key', { provider, key });
    } else {
      devMemoryKeyStore[provider] = key;
    }
  },

  async getApiKey(provider: ProviderId | string): Promise<string | null> {
    if (isTauriDesktop()) {
      const result = await invoke<string | null>('get_api_key', { provider });
      return result;
    } else {
      return devMemoryKeyStore[provider] || null;
    }
  },

  async hasApiKey(provider: ProviderId | string): Promise<boolean> {
    if (isTauriDesktop()) {
      const result = await invoke<boolean>('has_api_key', { provider });
      return result;
    } else {
      return Boolean(devMemoryKeyStore[provider] && devMemoryKeyStore[provider].length > 0);
    }
  },

  async deleteApiKey(provider: ProviderId | string): Promise<void> {
    if (isTauriDesktop()) {
      await invoke('delete_api_key', { provider });
    } else {
      delete devMemoryKeyStore[provider];
    }
  },

  async saveSettings(settings: AppSettings): Promise<void> {
    const jsonStr = JSON.stringify(settings, null, 2);
    if (isTauriDesktop()) {
      await invoke('save_settings', { settingsJson: jsonStr });
    } else {
      devSettingsMemory = settings;
    }
  },

  async loadSettings(): Promise<AppSettings | null> {
    if (isTauriDesktop()) {
      const jsonStr = await invoke<string | null>('load_settings');
      if (jsonStr) {
        try {
          return JSON.parse(jsonStr) as AppSettings;
        } catch {
          return null;
        }
      }
      return null;
    } else {
      return devSettingsMemory;
    }
  },

  async getSystemStatus(): Promise<{ status: string; os: string; arch: string; security_storage: string }> {
    if (isTauriDesktop()) {
      return await invoke('get_system_status');
    }
    return {
      status: 'ready (web dev preview)',
      os: 'windows',
      arch: 'x86_64',
      security_storage: 'Windows Credential Manager (Active in Tauri)',
    };
  },
};

export const windowControls = {
  async minimize(): Promise<void> {
    if (isTauriDesktop()) {
      try {
        await invoke('minimize_window');
      } catch (err) {
        console.error('Failed to minimize window:', err);
      }
    }
  },

  async toggleMaximize(): Promise<boolean> {
    if (isTauriDesktop()) {
      try {
        return await invoke<boolean>('toggle_maximize_window');
      } catch (err) {
        console.error('Failed to toggle maximize:', err);
      }
    }
    return false;
  },

  async isMaximized(): Promise<boolean> {
    if (isTauriDesktop()) {
      try {
        return await invoke<boolean>('is_window_maximized');
      } catch (err) {
        console.error('Failed to check maximized state:', err);
      }
    }
    return false;
  },

  async close(): Promise<void> {
    if (isTauriDesktop()) {
      try {
        await invoke('close_window');
      } catch (err) {
        console.error('Failed to close window:', err);
      }
    }
  },
};
