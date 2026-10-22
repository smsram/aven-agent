export type NavigationTab = 'new-chat' | 'activity' | 'plugins';

export type TaskStatus = 'running' | 'completed' | 'failed' | 'paused' | 'queued';

export interface TokenUsage {
  promptTokens: number;
  completionTokens: number;
  totalTokens: number;
}

export interface TaskArtifact {
  name: string;
  path: string;
  type: 'code' | 'diff' | 'markdown' | 'json';
  sizeBytes?: number;
}

export interface ChatMessage {
  id: string;
  sender: 'user' | 'assistant';
  text: string;
  timestamp: string;
  status?: 'thinking' | 'done' | 'error';
  logs?: { timestamp: string; level: 'info' | 'warn' | 'action' | 'error'; message: string }[];
}

export interface AgentTask {
  id: string;
  title: string;
  prompt: string;
  status: TaskStatus;
  createdAt: string;
  completedAt?: string;
  duration?: string;
  model: string;
  provider: string;
  output?: string;
  logs: { timestamp: string; level: 'info' | 'warn' | 'action' | 'error'; message: string }[];
  tokenUsage?: TokenUsage;
  artifacts?: TaskArtifact[];
  messages?: ChatMessage[];
}

export type ProviderId = 'anthropic' | 'openai' | 'gemini' | 'ollama' | 'groq' | 'mistral' | 'openrouter';

export type ConnectionStatus = 'connected' | 'disconnected' | 'configuring' | 'error';

export interface ModelProvider {
  id: ProviderId;
  name: string;
  description: string;
  status: ConnectionStatus;
  isLocal?: boolean;
  requiresKey: boolean;
  hasKey: boolean;
  defaultModel: string;
  availableModels: string[];
  endpoint?: string;
  docsUrl: string;
  latencyMs?: number;
}

export type IntegrationId = 'github' | 'slack' | 'linear' | 'jira' | 'terminal';

export interface Integration {
  id: IntegrationId;
  name: string;
  description: string;
  status: ConnectionStatus;
  category: 'developer' | 'issue-tracker' | 'communication' | 'system';
  configuredAccount?: string;
  lastSync?: string;
}

// MCP / Plugin interface
export interface McpPlugin {
  id: string;
  name: string;
  transport: 'stdio' | 'sse';
  command?: string;
  args?: string[];
  url?: string;
  env?: Record<string, string>;
  status: 'active' | 'disabled' | 'error';
  description?: string;
  createdAt: string;
}

export interface GeneralSettings {
  defaultWorkspace: string;
  runInBackground: boolean;
  launchAtStartup: boolean;
  confirmDangerousCommands: boolean;
  telemetryEnabled: boolean;
}

export interface AppearanceSettings {
  theme: 'dark-linear' | 'dark-midnight' | 'dark-slate';
  compactDensity: boolean;
  fontSize: 'compact' | 'default' | 'large';
  showStatusBar: boolean;
}

export interface ShortcutItem {
  id: string;
  label: string;
  category: 'general' | 'navigation' | 'task';
  keys: string[];
  description: string;
}

export interface PermissionsSettings {
  fileSystemAccess: 'read-write' | 'read-only' | 'prompt-each';
  shellExecution: 'auto-safe' | 'prompt-all' | 'disabled';
  networkAccess: 'full' | 'restricted' | 'offline';
  restrictedPaths: string[];
}

export interface MemorySettings {
  enableMemory: boolean;
  localVectorIndex: boolean;
  maxContextTokens: number;
  cacheSizeMb: number;
}

export interface ApiConfig {
  activeProvider: ProviderId;
  activeModel: string;
  temperature: number;
  maxTokens: number;
  customBaseUrl?: string;
}

export interface AppSettings {
  general: GeneralSettings;
  appearance: AppearanceSettings;
  api: ApiConfig;
  permissions: PermissionsSettings;
  memory: MemorySettings;
}
