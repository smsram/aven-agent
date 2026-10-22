import { ModelProvider, Integration } from '../types';
import { secureStore } from './tauriIpc';

const DEFAULT_PROVIDERS: ModelProvider[] = [
  {
    id: 'anthropic',
    name: 'Anthropic',
    description: 'Claude 3.7 Sonnet, Claude 3.5 Sonnet, and Claude 3.5 Haiku with hybrid reasoning.',
    status: 'connected',
    requiresKey: true,
    hasKey: true,
    defaultModel: 'Claude 3.7 Sonnet',
    availableModels: ['Claude 3.7 Sonnet', 'Claude 3.5 Sonnet', 'Claude 3.5 Haiku'],
    docsUrl: 'https://docs.anthropic.com',
    latencyMs: 142
  },
  {
    id: 'openai',
    name: 'OpenAI',
    description: 'GPT-4o, GPT-4o mini, and o3-mini reasoning models for autonomous execution.',
    status: 'connected',
    requiresKey: true,
    hasKey: true,
    defaultModel: 'GPT-4o',
    availableModels: ['GPT-4o', 'GPT-4o mini', 'o3-mini', 'o1'],
    docsUrl: 'https://platform.openai.com/docs',
    latencyMs: 198
  },
  {
    id: 'gemini',
    name: 'Google Gemini',
    description: 'Gemini 2.5 Pro and Gemini 2.0 Flash with expansive multimodal context windows.',
    status: 'disconnected',
    requiresKey: true,
    hasKey: false,
    defaultModel: 'Gemini 2.5 Pro',
    availableModels: ['Gemini 2.5 Pro', 'Gemini 2.0 Flash', 'Gemini 1.5 Pro'],
    docsUrl: 'https://ai.google.dev',
    latencyMs: 110
  },
  {
    id: 'ollama',
    name: 'Ollama (Local)',
    description: 'Run open-weight models on your local GPU without sending data across the network.',
    status: 'connected',
    isLocal: true,
    requiresKey: false,
    hasKey: true,
    defaultModel: 'DeepSeek-R1 (8B)',
    availableModels: ['DeepSeek-R1 (8B)', 'Llama 3.3 (70B)', 'Qwen 2.5 Coder (32B)', 'Mistral (7B)'],
    endpoint: 'http://localhost:11434',
    docsUrl: 'https://ollama.ai',
    latencyMs: 12
  },
  {
    id: 'groq',
    name: 'Groq Cloud',
    description: 'Ultra-low latency LPU inference for high-speed agentic loops.',
    status: 'disconnected',
    requiresKey: true,
    hasKey: false,
    defaultModel: 'Llama 3.3 70B Versatile',
    availableModels: ['Llama 3.3 70B Versatile', 'Mixtral 8x7B 32k'],
    docsUrl: 'https://groq.com',
    latencyMs: 45
  },
  {
    id: 'openrouter',
    name: 'OpenRouter',
    description: 'Unified gateway to over 100+ commercial and open-source models with failover routing.',
    status: 'disconnected',
    requiresKey: true,
    hasKey: false,
    defaultModel: 'Auto-Routing / Lowest Cost',
    availableModels: ['Anthropic / Claude 3.7 Sonnet', 'Meta / Llama 3.3 70B', 'DeepSeek / DeepSeek-V3'],
    docsUrl: 'https://openrouter.ai',
    latencyMs: 220
  }
];

const DEFAULT_INTEGRATIONS: Integration[] = [
  {
    id: 'github',
    name: 'GitHub',
    description: 'Inspect repositories, checkout branches, author pull requests and review diffs.',
    status: 'connected',
    category: 'developer',
    configuredAccount: 'github.com/aven-labs',
    lastSync: '10m ago'
  },
  {
    id: 'linear',
    name: 'Linear',
    description: 'Link tasks to Linear issues, auto-update cycle progress and post execution summaries.',
    status: 'connected',
    category: 'issue-tracker',
    configuredAccount: 'aven-core workspace',
    lastSync: '2m ago'
  },
  {
    id: 'terminal',
    name: 'Local Shell / PowerShell',
    description: 'Execute build scripts, test suites, and git commands in the Windows terminal.',
    status: 'connected',
    category: 'system',
    configuredAccount: 'PowerShell 7.5 (Sandbox)',
    lastSync: 'Active'
  },
  {
    id: 'slack',
    name: 'Slack',
    description: 'Receive notifications on task completion and interactive authorization prompts.',
    status: 'disconnected',
    category: 'communication'
  },
  {
    id: 'jira',
    name: 'Atlassian Jira',
    description: 'Bidirectional synchronization of sprints, epics, and work-item issue transitions.',
    status: 'disconnected',
    category: 'issue-tracker'
  }
];

class ConnectionStore {
  private providers: ModelProvider[] = [...DEFAULT_PROVIDERS];
  private integrations: Integration[] = [...DEFAULT_INTEGRATIONS];
  private listeners: (() => void)[] = [];

  constructor() {
    this.refreshKeyStatuses();
  }

  async refreshKeyStatuses(): Promise<void> {
    for (const p of this.providers) {
      if (p.requiresKey) {
        try {
          const has = await secureStore.hasApiKey(p.id);
          p.hasKey = has;
          if (has && p.status === 'disconnected') {
            p.status = 'connected';
          }
        } catch {
          // ignore error in check
        }
      }
    }
    this.notify();
  }

  getProviders(): ModelProvider[] {
    return [...this.providers];
  }

  getIntegrations(): Integration[] {
    return [...this.integrations];
  }

  toggleProviderStatus(id: string): void {
    this.providers = this.providers.map((p) => {
      if (p.id === id) {
        const nextStatus = p.status === 'connected' ? 'disconnected' : 'connected';
        return { ...p, status: nextStatus };
      }
      return p;
    });
    this.notify();
  }

  toggleIntegrationStatus(id: string): void {
    this.integrations = this.integrations.map((i) => {
      if (i.id === id) {
        const nextStatus = i.status === 'connected' ? 'disconnected' : 'connected';
        return {
          ...i,
          status: nextStatus,
          lastSync: nextStatus === 'connected' ? 'Just now' : undefined
        };
      }
      return i;
    });
    this.notify();
  }

  subscribe(listener: () => void): () => void {
    this.listeners.push(listener);
    return () => {
      this.listeners = this.listeners.filter((l) => l !== listener);
    };
  }

  private notify(): void {
    this.listeners.forEach((l) => l());
  }
}

export const connectionStore = new ConnectionStore();
