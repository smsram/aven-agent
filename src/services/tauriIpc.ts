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

export interface CredentialPairMetadata {
  pair_index: number;
  account_id_masked: string;
  is_valid: boolean;
}

export interface CredentialsStatus {
  configured_count: number;
  pairs: CredentialPairMetadata[];
  preferred_index: number;
  model_id: string;
}

export interface PairTestResult {
  pair_index: number;
  account_id_masked: string;
  status: 'connected' | 'failed';
  error_message?: string | null;
}

export interface OverallTestResult {
  all_tested: boolean;
  any_connected: boolean;
  results: PairTestResult[];
}

export interface ChatMessagePayload {
  role: 'system' | 'user' | 'assistant';
  content: string;
}

export interface StreamChunkPayload {
  conversation_id: string;
  message_id: string;
  chunk: string;
}

export interface StreamDonePayload {
  conversation_id: string;
  message_id: string;
  full_text: string;
  cancelled: boolean;
  error?: string | null;
  prompt_tokens?: number | null;
  completion_tokens?: number | null;
  total_tokens?: number | null;
}

export interface ChatActivityStatusPayload {
  conversation_id: string;
  message_id: string;
  status: 'searching' | 'synthesizing' | 'tool_running' | 'done';
  description: string;
  integration_id?: string;
  tool_name?: string;
}


export const cloudflareService = {
  async getStatus(): Promise<CredentialsStatus> {
    if (isTauriDesktop()) {
      return await invoke<CredentialsStatus>('get_cloudflare_status');
    }
    return {
      configured_count: 0,
      pairs: [],
      preferred_index: 0,
      model_id: '@cf/qwen/qwen3.8-27b',
    };
  },

  async saveCredentials(accountIds: string, apiKeys: string): Promise<CredentialsStatus> {
    if (isTauriDesktop()) {
      return await invoke<CredentialsStatus>('save_cloudflare_credentials', {
        accountIds,
        apiKeys,
      });
    }
    throw new Error('OS secure storage is only available in Tauri desktop application');
  },

  async testCredentials(): Promise<OverallTestResult> {
    if (isTauriDesktop()) {
      return await invoke<OverallTestResult>('test_cloudflare_credentials');
    }
    return {
      all_tested: true,
      any_connected: false,
      results: [],
    };
  },

  async clearCredentials(): Promise<void> {
    if (isTauriDesktop()) {
      await invoke('clear_cloudflare_credentials');
    }
  },

  async startChatStream(
    conversationId: string,
    messageId: string,
    messages: ChatMessagePayload[],
    maxTokens?: number,
    temperature?: number
  ): Promise<void> {
    if (isTauriDesktop()) {
      await invoke('start_chat_stream', {
        conversationId,
        messageId,
        messages,
        maxTokens: maxTokens ?? 4096,
        temperature: temperature ?? 0.2,
      });
    }
  },

  async stopChatStream(conversationId: string): Promise<void> {
    if (isTauriDesktop()) {
      await invoke('stop_chat_stream', { conversationId });
    }
  },
};

export const pluginsService = {
  async getPluginsState(): Promise<import('../types').PluginsStatePayload> {
    if (isTauriDesktop()) {
      return await invoke<import('../types').PluginsStatePayload>('get_plugins_state');
    }
    // Fallback for browser dev preview
    return {
      catalog: [],
      configured_plugins: [],
      capabilities: [],
      system_info: {
        os: 'windows',
        arch: 'x86_64',
        is_windows: true,
        platform_supported: true,
        features: { browser_opener: true, application_launcher: true, scoped_filesystem: true },
      },
    };
  },

  async savePluginConfig(
    config: import('../types').PluginConfig,
    secretKey?: string
  ): Promise<import('../types').PluginMetadata> {
    if (isTauriDesktop()) {
      return await invoke<import('../types').PluginMetadata>('save_plugin_config', {
        config,
        secretKey: secretKey || null,
      });
    }
    throw new Error('Plugins management requires Tauri desktop runtime');
  },

  async deletePluginConfig(pluginId: string): Promise<void> {
    if (isTauriDesktop()) {
      await invoke('delete_plugin_config', { pluginId });
    }
  },

  async testPluginConnection(pluginId: string): Promise<import('../types').TestConnectionResult> {
    if (isTauriDesktop()) {
      return await invoke<import('../types').TestConnectionResult>('test_plugin_connection', {
        pluginId,
      });
    }
    return {
      success: false,
      status: 'error',
      latency_ms: 0,
      message: 'Requires desktop environment',
    };
  },

  async connectPlugin(pluginId: string): Promise<import('../types').PluginMetadata> {
    if (isTauriDesktop()) {
      return await invoke<import('../types').PluginMetadata>('connect_plugin', { pluginId });
    }
    throw new Error('Requires desktop environment');
  },

  async disconnectPlugin(pluginId: string): Promise<import('../types').PluginMetadata> {
    if (isTauriDesktop()) {
      return await invoke<import('../types').PluginMetadata>('disconnect_plugin', { pluginId });
    }
    throw new Error('Requires desktop environment');
  },

  async executeCapability(
    request: import('../types').CapabilityExecutionRequest
  ): Promise<import('../types').CapabilityExecutionResult> {
    if (isTauriDesktop()) {
      return await invoke<import('../types').CapabilityExecutionResult>('execute_capability', {
        request,
      });
    }
    return {
      success: false,
      capability_id: request.capability_id,
      error: 'Desktop capability execution is available in Tauri desktop application',
    };
  },
};

export const chatStorageService = {
  async loadConversations(): Promise<import('../types').PersistedStorageFile> {
    if (isTauriDesktop()) {
      return await invoke<import('../types').PersistedStorageFile>('load_conversations');
    }
    return { version: 1, conversations: [] };
  },

  async saveConversation(conversation: import('../types').PersistedConversation): Promise<void> {
    if (isTauriDesktop()) {
      await invoke('save_conversation', { conversation });
    }
  },

  async deleteConversation(conversationId: string): Promise<void> {
    if (isTauriDesktop()) {
      await invoke('delete_conversation', { conversationId });
    }
  },

  async renameConversation(conversationId: string, title: string): Promise<void> {
    if (isTauriDesktop()) {
      await invoke('rename_conversation', { conversationId, title });
    }
  },

  async saveActiveConversationId(conversationId: string | null): Promise<void> {
    if (isTauriDesktop()) {
      await invoke('save_active_conversation_id', { conversationId });
    }
  },

  async clearAllConversations(): Promise<void> {
    if (isTauriDesktop()) {
      await invoke('clear_all_conversations');
    }
  },
};



