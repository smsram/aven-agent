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

export interface ProcessStep {
  id: string;
  label: string;
  status: 'pending' | 'running' | 'completed' | 'failed' | 'cancelled';
  integration_id?: string;
  tool_name?: string;
  timestamp: string;
  error?: string;
}

export interface MessageMetadata {
  model?: string;
  prompt_tokens?: number;
  completion_tokens?: number;
  total_tokens?: number;
  duration_ms?: number;
  duration_formatted?: string;
}

export interface ChatMessage {
  id: string;
  sender: 'user' | 'assistant';
  text: string;
  timestamp: string;
  status?: 'thinking' | 'done' | 'error' | 'interrupted';
  activityStatus?: {
    status: 'searching' | 'synthesizing' | 'tool_running' | 'done';
    description: string;
    integration_id?: string;
    tool_name?: string;
  };
  processSteps?: ProcessStep[];
  process_steps?: ProcessStep[];
  metadata?: MessageMetadata;
  logs?: { timestamp: string; level: 'info' | 'warn' | 'action' | 'error'; message: string }[];
}

export interface PersistedConversation {
  id: string;
  title: string;
  prompt: string;
  status: string;
  created_at: string;
  completed_at?: string;
  duration?: string;
  model: string;
  provider: string;
  output?: string;
  token_usage?: TokenUsage;
  logs: { timestamp: string; level: 'info' | 'warn' | 'action' | 'error'; message: string }[];
  messages: {
    id: string;
    sender: string;
    text: string;
    timestamp: string;
    status: string;
    metadata?: MessageMetadata;
    process_steps: ProcessStep[];
    logs?: { timestamp: string; level: 'info' | 'warn' | 'action' | 'error'; message: string }[];
  }[];
  updated_at: number;
}

export interface PersistedStorageFile {
  version: number;
  active_conversation_id?: string;
  conversations: PersistedConversation[];
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

export type ProviderId = 'cloudflare' | 'anthropic' | 'openai' | 'gemini' | 'ollama' | 'groq' | 'mistral' | 'openrouter';

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

// ------------------ Plugins Subsystem Types ------------------
export type PluginKind = 'mcp' | 'api' | 'native';

export type PluginStatus =
  | 'not_configured'
  | 'disconnected'
  | 'connecting'
  | 'connected'
  | 'needs_permission'
  | 'error'
  | 'unsupported';

export interface McpToolDefinition {
  name: string;
  description?: string;
  input_schema: Record<string, unknown>;
}

export interface CatalogEntry {
  id: string;
  kind: PluginKind;
  name: string;
  category: string;
  description: string;
  supported_capabilities: string[];
  requirements: string;
  setup_instructions: string;
  is_implemented: boolean;
  default_command?: string;
  default_args?: string[];
  default_url?: string;
  permissions_notice: string;
}

export interface PluginMetadata {
  id: string;
  kind: PluginKind;
  name: string;
  status: PluginStatus;
  has_secret: boolean;
  error_message?: string;
  latency_ms?: number;
  approved_paths: string[];
  settings: Record<string, string>;
  discovered_tools: McpToolDefinition[];
}

export interface PluginConfig {
  id: string;
  kind: PluginKind;
  name: string;
  enabled: boolean;
  transport?: string;
  command?: string;
  args?: string[];
  url?: string;
  approved_paths?: string[];
  settings: Record<string, string>;
  last_status: PluginStatus;
  last_error?: string;
}

export interface RegisteredCapability {
  id: string;
  source_id: string;
  kind: PluginKind;
  name: string;
  description: string;
  input_schema: Record<string, unknown>;
  required_permission: 'read' | 'write' | 'execute' | 'network' | 'full';
  requires_confirmation: boolean;
  is_available: boolean;
}

export interface PluginsStatePayload {
  catalog: CatalogEntry[];
  configured_plugins: PluginMetadata[];
  capabilities: RegisteredCapability[];
  system_info: {
    os: string;
    arch: string;
    is_windows: boolean;
    platform_supported: boolean;
    features: Record<string, boolean>;
  };
}

export interface TestConnectionResult {
  success: boolean;
  status: PluginStatus;
  latency_ms: number;
  message: string;
}

export interface CapabilityExecutionRequest {
  capability_id: string;
  arguments: Record<string, unknown>;
  user_confirmed: boolean;
}

export interface CapabilityExecutionResult {
  success: boolean;
  capability_id: string;
  result?: Record<string, unknown>;
  error?: string;
}

// Backward compatibility McpPlugin
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
