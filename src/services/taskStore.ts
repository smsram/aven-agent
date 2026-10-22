import { AgentTask, ChatMessage, TaskStatus } from '../types';

const INITIAL_TASKS: AgentTask[] = [
  {
    id: 'tsk_01jk98a',
    title: 'Audit repository dependencies and security vulnerabilities',
    prompt: 'Scan Cargo.toml and package.json for outdated or insecure packages, analyze dependency graph, and report recommendations.',
    status: 'completed',
    createdAt: '12 minutes ago',
    completedAt: '10 minutes ago',
    duration: '1m 42s',
    model: 'Claude 3.7 Sonnet',
    provider: 'Anthropic',
    output: `Found 0 critical or high vulnerabilities in cargo dependency tree.\nFound 0 npm vulnerabilities across 29 packages.\nKey recommendations:\n- Tauri 2.12.5 dependencies are on the latest stable channel.\n- keyring 4.2.0 is configured with Windows Credential Manager.\n- Recommended adding Content-Security-Policy header in tauri.conf.json before production release.`,
    logs: [
      { timestamp: '14:40:02', level: 'info', message: 'Task initialized with model Claude 3.7 Sonnet' },
      { timestamp: '14:40:05', level: 'action', message: 'Reading Cargo.lock and package-lock.json' },
      { timestamp: '14:40:48', level: 'info', message: 'Checked 480 locked crates against advisory database' },
      { timestamp: '14:41:40', level: 'action', message: 'Generated security report and dependency matrix' },
      { timestamp: '14:41:44', level: 'info', message: 'Task finalized successfully' }
    ],
    tokenUsage: {
      promptTokens: 1420,
      completionTokens: 388,
      totalTokens: 1808
    },
    artifacts: [
      { name: 'audit_summary.md', path: 'artifacts/audit_summary.md', type: 'markdown', sizeBytes: 2450 }
    ],
    messages: [
      {
        id: 'msg_01a',
        sender: 'user',
        text: 'Scan Cargo.toml and package.json for outdated or insecure packages, analyze dependency graph, and report recommendations.',
        timestamp: '14:40:00'
      },
      {
        id: 'msg_01b',
        sender: 'assistant',
        text: `Found 0 critical or high vulnerabilities in cargo dependency tree.\nFound 0 npm vulnerabilities across 29 packages.\n\nKey recommendations:\n- Tauri 2.12.5 dependencies are on the latest stable channel.\n- keyring 4.2.0 is configured with Windows Credential Manager.\n- Recommended adding Content-Security-Policy header in tauri.conf.json before production release.`,
        status: 'done',
        timestamp: '14:41:44',
        logs: [
          { timestamp: '14:40:02', level: 'info', message: 'Checked 480 crates against RustSec advisory' },
          { timestamp: '14:41:40', level: 'action', message: 'Generated security report' }
        ]
      }
    ]
  },
  {
    id: 'tsk_01jk98b',
    title: 'Verify Windows Credential Manager keyring bindings',
    prompt: 'Check Tauri Rust invoke handlers for set_api_key, get_api_key, and has_api_key to ensure DPAPI credentials never leak to frontend.',
    status: 'completed',
    createdAt: '1 hour ago',
    completedAt: '58 minutes ago',
    duration: '48s',
    model: 'GPT-4o',
    provider: 'OpenAI',
    output: `Verified: Keyring integration uses OS-native Windows Credential Manager.\n- Secrets are kept exclusively in OS protected memory\n- Frontend receives boolean existence status rather than plain text keys\n- Zero localStorage or unencrypted disk exposure detected.`,
    logs: [
      { timestamp: '13:52:10', level: 'info', message: 'Inspecting src-tauri/src/lib.rs handler signatures' },
      { timestamp: '13:52:25', level: 'action', message: 'Tracing memory flow of Entry::new(SERVICE_NAME)' },
      { timestamp: '13:52:58', level: 'info', message: 'Credential isolation criteria satisfied' }
    ],
    tokenUsage: {
      promptTokens: 860,
      completionTokens: 210,
      totalTokens: 1070
    },
    messages: [
      {
        id: 'msg_02a',
        sender: 'user',
        text: 'Check Tauri Rust invoke handlers for set_api_key, get_api_key, and has_api_key to ensure DPAPI credentials never leak to frontend.',
        timestamp: '13:52:00'
      },
      {
        id: 'msg_02b',
        sender: 'assistant',
        text: `Verified: Keyring integration uses OS-native Windows Credential Manager.\n- Secrets are kept exclusively in OS protected memory\n- Frontend receives boolean existence status rather than plain text keys\n- Zero localStorage or unencrypted disk exposure detected.`,
        status: 'done',
        timestamp: '13:52:58'
      }
    ]
  },
  {
    id: 'tsk_01jk98c',
    title: 'Generate TypeScript client interfaces for Tauri backend',
    prompt: 'Create typed contracts for all Tauri commands, responses, and settings payloads to maintain end-to-end type safety.',
    status: 'completed',
    createdAt: '3 hours ago',
    completedAt: '2 hours 58m ago',
    duration: '2m 14s',
    model: 'Claude 3.7 Sonnet',
    provider: 'Anthropic',
    output: `Successfully mapped all 7 Tauri commands to TypeScript definitions in src/types/index.ts and implemented service wrapper in src/services/tauriIpc.ts.`,
    logs: [
      { timestamp: '11:15:00', level: 'info', message: 'Extracted Rust structs: KeyStatus, AppSettings' },
      { timestamp: '11:16:10', level: 'action', message: 'Generated src/types/index.ts' },
      { timestamp: '11:17:14', level: 'info', message: 'Typecheck passed with zero compiler diagnostics' }
    ],
    tokenUsage: {
      promptTokens: 2310,
      completionTokens: 520,
      totalTokens: 2830
    },
    artifacts: [
      { name: 'index.ts', path: 'src/types/index.ts', type: 'code', sizeBytes: 2890 }
    ],
    messages: [
      {
        id: 'msg_03a',
        sender: 'user',
        text: 'Create typed contracts for all Tauri commands, responses, and settings payloads to maintain end-to-end type safety.',
        timestamp: '11:15:00'
      },
      {
        id: 'msg_03b',
        sender: 'assistant',
        text: `Successfully mapped all 7 Tauri commands to TypeScript definitions in src/types/index.ts and implemented service wrapper in src/services/tauriIpc.ts.`,
        status: 'done',
        timestamp: '11:17:14'
      }
    ]
  }
];

class TaskStore {
  private tasks: AgentTask[] = [...INITIAL_TASKS];
  private listeners: (() => void)[] = [];

  getTasks(): AgentTask[] {
    return [...this.tasks];
  }

  getRecentTasks(limit = 4): AgentTask[] {
    return this.tasks.slice(0, limit);
  }

  getTaskById(id: string): AgentTask | undefined {
    return this.tasks.find((t) => t.id === id);
  }

  createTask(title: string, prompt: string, model: string, provider: string): AgentTask {
    const id = `tsk_${Date.now().toString(36)}`;
    const nowTime = new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    const userMsgId = `msg_u_${Date.now().toString(36)}`;
    const asstMsgId = `msg_a_${Date.now().toString(36)}`;

    const userMessage: ChatMessage = {
      id: userMsgId,
      sender: 'user',
      text: prompt,
      timestamp: nowTime,
    };

    const assistantMessage: ChatMessage = {
      id: asstMsgId,
      sender: 'assistant',
      text: '',
      status: 'thinking',
      timestamp: nowTime,
      logs: [
        { timestamp: new Date().toLocaleTimeString(), level: 'info', message: `Initializing workspace context with ${model}` }
      ]
    };

    const newTask: AgentTask = {
      id,
      title: title || (prompt.length > 55 ? prompt.slice(0, 52) + '...' : prompt),
      prompt,
      status: 'running',
      createdAt: 'Just now',
      model,
      provider,
      logs: [
        { timestamp: new Date().toLocaleTimeString(), level: 'info', message: `Task queued for execution on ${model}` }
      ],
      tokenUsage: {
        promptTokens: Math.floor(prompt.length * 1.3),
        completionTokens: 0,
        totalTokens: Math.floor(prompt.length * 1.3)
      },
      messages: [userMessage, assistantMessage]
    };

    this.tasks = [newTask, ...this.tasks];
    this.notify();

    // Progressive real-time steps with realistic response
    setTimeout(() => {
      this.appendLogToAssistant(id, asstMsgId, {
        timestamp: new Date().toLocaleTimeString(),
        level: 'action',
        message: 'Analyzing workspace structure and files'
      });
    }, 450);

    setTimeout(() => {
      this.appendLogToAssistant(id, asstMsgId, {
        timestamp: new Date().toLocaleTimeString(),
        level: 'info',
        message: 'Synthesizing recommendations'
      });
    }, 1100);

    setTimeout(() => {
      const responseText = this.generateResponse(prompt, model);
      this.finishAssistantMessage(id, asstMsgId, responseText);
    }, 1900);

    return newTask;
  }

  appendUserMessage(taskId: string, text: string, model: string): void {
    const nowTime = new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    const userMsgId = `msg_u_${Date.now().toString(36)}`;
    const asstMsgId = `msg_a_${Date.now().toString(36)}`;

    const userMsg: ChatMessage = {
      id: userMsgId,
      sender: 'user',
      text,
      timestamp: nowTime,
    };

    const asstMsg: ChatMessage = {
      id: asstMsgId,
      sender: 'assistant',
      text: '',
      status: 'thinking',
      timestamp: nowTime,
      logs: [
        { timestamp: new Date().toLocaleTimeString(), level: 'info', message: 'Processing follow-up instruction' }
      ]
    };

    this.tasks = this.tasks.map((t) => {
      if (t.id === taskId) {
        return {
          ...t,
          status: 'running',
          messages: [...(t.messages || []), userMsg, asstMsg],
        };
      }
      return t;
    });
    this.notify();

    setTimeout(() => {
      const responseText = this.generateResponse(text, model);
      this.finishAssistantMessage(taskId, asstMsgId, responseText);
    }, 1600);
  }

  private appendLogToAssistant(taskId: string, asstMsgId: string, log: { timestamp: string; level: 'info' | 'warn' | 'action' | 'error'; message: string }) {
    this.tasks = this.tasks.map((t) => {
      if (t.id === taskId) {
        const updatedMessages = (t.messages || []).map((m) => {
          if (m.id === asstMsgId) {
            return {
              ...m,
              logs: [...(m.logs || []), log]
            };
          }
          return m;
        });
        return { ...t, messages: updatedMessages, logs: [...t.logs, log] };
      }
      return t;
    });
    this.notify();
  }

  private finishAssistantMessage(taskId: string, asstMsgId: string, responseText: string) {
    this.tasks = this.tasks.map((t) => {
      if (t.id === taskId) {
        const updatedMessages = (t.messages || []).map((m) => {
          if (m.id === asstMsgId) {
            return {
              ...m,
              text: responseText,
              status: 'done' as const,
              timestamp: new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
            };
          }
          return m;
        });

        return {
          ...t,
          status: 'completed' as TaskStatus,
          completedAt: 'Just now',
          duration: '1.9s',
          output: responseText,
          messages: updatedMessages,
          tokenUsage: {
            promptTokens: (t.tokenUsage?.promptTokens || 120),
            completionTokens: Math.floor(responseText.length * 0.9),
            totalTokens: (t.tokenUsage?.promptTokens || 120) + Math.floor(responseText.length * 0.9)
          }
        };
      }
      return t;
    });
    this.notify();
  }

  private generateResponse(prompt: string, model: string): string {
    const lower = prompt.toLowerCase();
    if (lower.includes('audit') || lower.includes('security') || lower.includes('vulnerabilit')) {
      return `Security audit completed using ${model}:\n\n1. Dependencies Verified:\n   - Cargo crates: 480 locked, 0 vulnerabilities flagged\n   - NPM packages: 29 audited, 0 vulnerabilities\n2. Findings:\n   - Native Windows Credential Manager is active for key isolation\n   - Tauri 2 custom title bar controls configured with correct drag regions\n   - Context menus and webview navigation shortcuts secured.`;
    }
    if (lower.includes('refactor') || lower.includes('clean') || lower.includes('code')) {
      return `Completed code inspection with ${model}:\n\n- Verified modular component tree (Sidebar, TopBar, ConversationView, SettingsModal)\n- TypeScript interfaces consolidated in src/types/index.ts\n- Zero unused variables or lint warnings detected.`;
    }
    return `Task processed successfully by ${model}.\n\nExecuted inspection on local workspace:\n- All operations verified against project guidelines\n- State saved to persistent task history\n- Ready for subsequent commands.`;
  }

  deleteTask(id: string): void {
    this.tasks = this.tasks.filter((t) => t.id !== id);
    this.notify();
  }

  clearAllTasks(): void {
    this.tasks = [];
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

export const taskStore = new TaskStore();
