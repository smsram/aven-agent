import {
  AgentTask,
  ChatMessage,
  TaskStatus,
  ProcessStep,
  MessageMetadata,
  PersistedConversation,
} from '../types';
import {
  cloudflareService,
  chatStorageService,
  ChatMessagePayload,
  StreamChunkPayload,
  StreamDonePayload,
  ChatActivityStatusPayload,
  isTauriDesktop,
} from './tauriIpc';
import { listen } from '@tauri-apps/api/event';

const SYSTEM_PROMPT =
  'You are Aven, a sophisticated engineering AI assistant. Provide concise, helpful, and well-structured answers using clean Markdown formatting.';

export function sanitizeProgressDescription(desc?: string): string {
  if (!desc) return 'Working';
  const trimmed = desc.trim();
  if (!trimmed) return 'Working';

  const lower = trimmed.toLowerCase();
  if (
    lower.includes('<!doctype') ||
    lower.includes('<html') ||
    lower.includes('<body') ||
    lower.includes('<div') ||
    lower.includes('<svg') ||
    lower.includes('<script') ||
    lower.includes('<style') ||
    (trimmed.startsWith('{') && trimmed.endsWith('}')) ||
    (trimmed.startsWith('[') && trimmed.endsWith(']'))
  ) {
    return 'Analyzing screen';
  }

  const clean = trimmed.replace(/<[^>]*>/g, '').trim();
  const firstLine = clean.split(/[\r\n]+/)[0]?.trim() || 'Working';

  if (firstLine.includes('/') || firstLine.includes('\\')) {
    if (firstLine.toLowerCase().endsWith('.html') || firstLine.toLowerCase().endsWith('.js') || firstLine.toLowerCase().endsWith('.ts')) {
      return 'Accessing project file';
    }
  }

  return firstLine.length > 50 ? firstLine.slice(0, 47) + '...' : firstLine;
}

import { sanitizeAssistantVisibleText } from '../components/chat/markdownUtils';
export { sanitizeAssistantVisibleText };


function toPersistedConversation(task: AgentTask): PersistedConversation {
  return {
    id: task.id,
    title: task.title,
    prompt: task.prompt,
    status: task.status,
    created_at: task.createdAt,
    completed_at: task.completedAt,
    duration: task.duration,
    model: task.model,
    provider: task.provider,
    output: task.output,
    token_usage: task.tokenUsage,
    logs: task.logs || [],
    messages: (task.messages || []).map((m) => ({
      id: m.id,
      sender: m.sender,
      text: m.text,
      timestamp: m.timestamp,
      status: m.status || (m.sender === 'user' ? 'done' : 'thinking'),
      metadata: m.metadata,
      process_steps: m.processSteps || m.process_steps || [],
      logs: m.logs || [],
    })),
    updated_at: Date.now(),
  };
}

function fromPersistedConversation(p: PersistedConversation): AgentTask {
  return {
    id: p.id,
    title: p.title,
    prompt: p.prompt,
    status: (p.status as TaskStatus) || 'completed',
    createdAt: p.created_at,
    completedAt: p.completed_at,
    duration: p.duration,
    model: p.model,
    provider: p.provider,
    output: p.output,
    tokenUsage: p.token_usage,
    logs: p.logs || [],
    messages: (p.messages || []).map((m) => {
      const isInterrupted = m.status === 'interrupted' || (m.sender === 'assistant' && m.status === 'thinking');
      const finalStatus = isInterrupted ? ('error' as const) : ((m.status as 'thinking' | 'done' | 'error' | 'interrupted') || 'done');
      const processSteps = (m.process_steps || []).map((s) => ({
        ...s,
        status: isInterrupted && s.status === 'running' ? ('cancelled' as const) : s.status,
      }));

      return {
        id: m.id,
        sender: m.sender as 'user' | 'assistant',
        text: m.text || (isInterrupted ? 'Generation was interrupted when the application was closed.' : ''),
        timestamp: m.timestamp,
        status: finalStatus,
        metadata: m.metadata,
        processSteps,
        process_steps: processSteps,
        logs: m.logs || [],
      };
    }),
  };
}

class TaskStore {
  private tasks: AgentTask[] = [];
  private listeners: (() => void)[] = [];
  private isListening = false;
  private isInitialized = false;
  private activeTaskId: string | null = null;
  private debounceSaveTimers = new Map<string, ReturnType<typeof setTimeout>>();
  private generationTimers = new Map<string, number>();

  constructor() {
    this.initEventListeners();
  }

  async initStore(): Promise<string | null> {
    if (this.isInitialized) {
      return this.activeTaskId;
    }
    this.isInitialized = true;

    try {
      const stored = await chatStorageService.loadConversations();
      if (stored && stored.conversations && stored.conversations.length > 0) {
        this.tasks = stored.conversations.map(fromPersistedConversation);
      }
      this.activeTaskId = null;
      this.notify();
      return null;
    } catch (err) {
      console.error('Failed to load persisted conversations on startup:', err);
      return null;
    }
  }

  private async initEventListeners() {
    if (this.isListening || !isTauriDesktop()) return;
    this.isListening = true;

    try {
      await listen<StreamChunkPayload>('chat-stream-chunk', (event) => {
        this.handleStreamChunk(event.payload);
      });

      await listen<StreamDonePayload>('chat-stream-done', (event) => {
        this.handleStreamDone(event.payload);
      });

      await listen<ChatActivityStatusPayload>('chat-activity-status', (event) => {
        this.handleActivityStatus(event.payload);
      });

      await listen<{ shortcut: string; timestamp: number }>('agent-emergency-stopped', (event) => {
        console.warn('Emergency global stop received from OS hotkey:', event.payload);
        if (this.activeTaskId) {
          this.stopGenerating(this.activeTaskId);
        } else {
          for (const t of this.tasks) {
            if (t.status === 'running') {
              this.stopGenerating(t.id);
            }
          }
        }
      });

      if (typeof window !== 'undefined') {
        window.addEventListener('keydown', (e) => {
          if (e.ctrlKey && e.shiftKey && e.key === 'Backspace') {
            if (this.activeTaskId) {
              this.stopGenerating(this.activeTaskId);
            } else {
              for (const t of this.tasks) {
                if (t.status === 'running') {
                  this.stopGenerating(t.id);
                }
              }
            }
          }
        });
      }
    } catch (err) {
      console.error('Failed to register Tauri chat stream listeners:', err);
    }
  }

  getTasks(): AgentTask[] {
    return [...this.tasks];
  }

  getRecentTasks(limit = 4): AgentTask[] {
    return this.tasks.slice(0, limit);
  }

  getTaskById(id: string): AgentTask | undefined {
    return this.tasks.find((t) => t.id === id);
  }

  getActiveTaskId(): string | null {
    return this.activeTaskId;
  }

  setActiveTaskId(id: string | null): void {
    this.activeTaskId = id;
    chatStorageService.saveActiveConversationId(id).catch((err) => {
      console.error('Failed to save active conversation id:', err);
    });
  }

  private persistTask(task: AgentTask): Promise<void> {
    const persisted = toPersistedConversation(task);
    return chatStorageService.saveConversation(persisted).catch((err) => {
      console.error(`Failed to persist conversation [${task.id}]:`, err);
    });
  }

  private scheduleDebouncedSave(task: AgentTask, delayMs = 400): void {
    const existing = this.debounceSaveTimers.get(task.id);
    if (existing) {
      clearTimeout(existing);
    }
    const timer = setTimeout(() => {
      this.debounceSaveTimers.delete(task.id);
      this.persistTask(task);
    }, delayMs);
    this.debounceSaveTimers.set(task.id, timer);
  }

  private cancelDebouncedSave(taskId: string): void {
    const timer = this.debounceSaveTimers.get(taskId);
    if (timer) {
      clearTimeout(timer);
      this.debounceSaveTimers.delete(taskId);
    }
  }

  createTask(title: string, prompt: string, model: string, provider: string): AgentTask {
    const id = `tsk_${Date.now()}_${Math.random().toString(36).substring(2, 7)}`;
    const nowTime = new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    const userMsgId = `msg_u_${Date.now()}`;
    const asstMsgId = `msg_a_${Date.now()}`;

    // Record monotonic generation start time
    this.generationTimers.set(asstMsgId, performance.now());

    const userMessage: ChatMessage = {
      id: userMsgId,
      sender: 'user',
      text: prompt,
      timestamp: nowTime,
      status: 'done',
    };

    const assistantMessage: ChatMessage = {
      id: asstMsgId,
      sender: 'assistant',
      text: '',
      status: 'thinking',
      timestamp: nowTime,
      processSteps: [],
      process_steps: [],
      logs: [
        {
          timestamp: new Date().toLocaleTimeString(),
          level: 'info',
          message: `Connected to Cloudflare Workers AI (${model})`,
        },
      ],
    };

    const newTask: AgentTask = {
      id,
      title: title || (prompt.length > 55 ? prompt.slice(0, 52) + '...' : prompt),
      prompt,
      status: 'running',
      createdAt: 'Just now',
      model,
      provider: provider || 'Cloudflare Workers AI',
      logs: [
        {
          timestamp: new Date().toLocaleTimeString(),
          level: 'info',
          message: `Stream initiated on ${model}`,
        },
      ],
      messages: [userMessage, assistantMessage],
    };

    this.tasks = [newTask, ...this.tasks];
    this.setActiveTaskId(id);
    this.notify();

    // Persist immediately so prompt & initial state are saved
    this.persistTask(newTask);

    // Start streaming request to Cloudflare Workers AI
    const messagesPayload: ChatMessagePayload[] = [
      { role: 'system', content: SYSTEM_PROMPT },
      { role: 'user', content: prompt },
    ];

    cloudflareService.startChatStream(id, asstMsgId, messagesPayload, model).catch((err) => {
      this.handleStreamDone({
        conversation_id: id,
        message_id: asstMsgId,
        full_text: '',
        cancelled: false,
        error: String(err),
      });
    });

    return newTask;
  }

  appendUserMessage(taskId: string, text: string, model: string): void {
    const task = this.getTaskById(taskId);
    if (!task) return;

    const nowTime = new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    const userMsgId = `msg_u_${Date.now()}`;
    const asstMsgId = `msg_a_${Date.now()}`;

    // Record monotonic generation start time
    this.generationTimers.set(asstMsgId, performance.now());

    const userMsg: ChatMessage = {
      id: userMsgId,
      sender: 'user',
      text,
      timestamp: nowTime,
      status: 'done',
    };

    const asstMsg: ChatMessage = {
      id: asstMsgId,
      sender: 'assistant',
      text: '',
      status: 'thinking',
      timestamp: nowTime,
      processSteps: [],
      process_steps: [],
      logs: [
        {
          timestamp: new Date().toLocaleTimeString(),
          level: 'info',
          message: `Sending message to ${model}`,
        },
      ],
    };

    const existingMessages = (task.messages || []).slice(-20);
    const historyPayload: ChatMessagePayload[] = [
      { role: 'system', content: SYSTEM_PROMPT },
    ];

    for (const m of existingMessages) {
      if (m.text && m.status !== 'error') {
        historyPayload.push({
          role: m.sender === 'user' ? 'user' : 'assistant',
          content: m.text,
        });
      }
    }
    historyPayload.push({ role: 'user', content: text });

    let updatedTask: AgentTask | null = null;
    this.tasks = this.tasks.map((t) => {
      if (t.id === taskId) {
        updatedTask = {
          ...t,
          status: 'running',
          model,
          messages: [...(t.messages || []), userMsg, asstMsg],
        };
        return updatedTask;
      }
      return t;
    });

    this.notify();
    if (updatedTask) {
      this.persistTask(updatedTask);
    }

    cloudflareService.startChatStream(taskId, asstMsgId, historyPayload, model).catch((err) => {
      this.handleStreamDone({
        conversation_id: taskId,
        message_id: asstMsgId,
        full_text: '',
        cancelled: false,
        error: String(err),
      });
    });
  }

  retryMessage(taskId: string, assistantMsgId: string): void {
    const task = this.getTaskById(taskId);
    if (!task || !task.messages) return;

    const asstIndex = task.messages.findIndex((m) => m.id === assistantMsgId);
    if (asstIndex === -1) return;

    // Record monotonic generation start time
    this.generationTimers.set(assistantMsgId, performance.now());

    const previousMessages = task.messages.slice(0, asstIndex);
    const historyPayload: ChatMessagePayload[] = [
      { role: 'system', content: SYSTEM_PROMPT },
    ];

    for (const m of previousMessages) {
      if (m.text && m.status !== 'error') {
        historyPayload.push({
          role: m.sender === 'user' ? 'user' : 'assistant',
          content: m.text,
        });
      }
    }

    let updatedTask: AgentTask | null = null;
    this.tasks = this.tasks.map((t) => {
      if (t.id === taskId) {
        const updated = (t.messages || []).map((m) => {
          if (m.id === assistantMsgId) {
            return {
              ...m,
              text: '',
              status: 'thinking' as const,
              processSteps: [],
              process_steps: [],
              activityStatus: undefined,
              logs: [
                {
                  timestamp: new Date().toLocaleTimeString(),
                  level: 'info' as const,
                  message: 'Retrying request with Cloudflare Workers AI...',
                },
              ],
            };
          }
          return m;
        });
        updatedTask = { ...t, status: 'running' as TaskStatus, messages: updated };
        return updatedTask;
      }
      return t;
    });

    this.notify();
    if (updatedTask) {
      this.persistTask(updatedTask);
    }

    const targetModel = task.model || '@cf/google/gemma-4-26b-a4b-it';
    cloudflareService.startChatStream(taskId, assistantMsgId, historyPayload, targetModel).catch((err) => {
      this.handleStreamDone({
        conversation_id: taskId,
        message_id: assistantMsgId,
        full_text: '',
        cancelled: false,
        error: String(err),
      });
    });
  }

  stopGenerating(taskId: string): void {
    cloudflareService.stopChatStream(taskId);
    this.cancelDebouncedSave(taskId);

    let updatedTask: AgentTask | null = null;
    this.tasks = this.tasks.map((t) => {
      if (t.id === taskId) {
        const updated = (t.messages || []).map((m) => {
          if (m.status === 'thinking') {
            const start = this.generationTimers.get(m.id);
            let durationMs: number | undefined;
            let durationFormatted: string | undefined;
            if (start !== undefined) {
              durationMs = Math.round(performance.now() - start);
              this.generationTimers.delete(m.id);
              durationFormatted = `${(durationMs / 1000).toFixed(1)}s`;
            }

            const steps = (m.processSteps || m.process_steps || []).map((s) => ({
              ...s,
              status: s.status === 'running' ? ('cancelled' as const) : s.status,
            }));

            const metadata: MessageMetadata = {
              model: t.model,
              duration_ms: durationMs,
              duration_formatted: durationFormatted,
            };

            return {
              ...m,
              status: 'done' as const,
              activityStatus: undefined,
              processSteps: steps,
              process_steps: steps,
              metadata,
              logs: [
                ...(m.logs || []),
                {
                  timestamp: new Date().toLocaleTimeString(),
                  level: 'warn' as const,
                  message: 'Generation cancelled by user',
                },
              ],
            };
          }
          return m;
        });

        updatedTask = {
          ...t,
          status: 'completed' as TaskStatus,
          messages: updated,
        };
        return updatedTask;
      }
      return t;
    });

    this.notify();
    if (updatedTask) {
      this.persistTask(updatedTask);
    }
  }

  handleStreamChunk(payload: StreamChunkPayload): void {
    if (!payload || !payload.conversation_id || !payload.message_id) return;
    const chunkText = payload.chunk || '';
    if (!chunkText) return;

    let targetTask: AgentTask | null = null;

    this.tasks = this.tasks.map((t) => {
      if (t.id === payload.conversation_id) {
        const updatedMessages = (t.messages || []).map((m) => {
          if (m.id === payload.message_id) {
            const combined = (m.text || '') + chunkText;
            return {
              ...m,
              text: sanitizeAssistantVisibleText(combined),
              status: 'thinking' as const,
            };
          }
          return m;
        });

        targetTask = {
          ...t,
          output: updatedMessages[updatedMessages.length - 1]?.text || t.output,
          messages: updatedMessages,
        };
        return targetTask;
      }
      return t;
    });

    if (targetTask) {
      this.notify();
      // Debounced write during streaming so partial generation is safely saved
      this.scheduleDebouncedSave(targetTask);
    }
  }

  handleActivityStatus(payload: ChatActivityStatusPayload): void {
    if (!payload || !payload.conversation_id || !payload.message_id) return;
    const sanitizedDescription = sanitizeProgressDescription(payload.description);
    let targetTask: AgentTask | null = null;

    this.tasks = this.tasks.map((t) => {
      if (t.id === payload.conversation_id) {
        const updatedMessages = (t.messages || []).map((m) => {
          if (m.id === payload.message_id) {
            const currentSteps: ProcessStep[] = [...(m.processSteps || m.process_steps || [])];

            if (payload.status === 'done') {
              // Complete running steps
              for (const s of currentSteps) {
                if (s.status === 'running') {
                  s.status = 'completed';
                }
              }
            } else {
              // Mark previous running steps as completed
              for (const s of currentSteps) {
                if (s.status === 'running' && s.label !== sanitizedDescription) {
                  s.status = 'completed';
                }
              }

              // Check if step with this description or tool_name exists
              const existingIndex = currentSteps.findIndex(
                (s) => s.label === sanitizedDescription || (payload.tool_name && s.tool_name === payload.tool_name)
              );

              if (existingIndex >= 0) {
                currentSteps[existingIndex] = {
                  ...currentSteps[existingIndex],
                  status: 'running',
                  label: sanitizedDescription,
                  integration_id: payload.integration_id || currentSteps[existingIndex].integration_id,
                  tool_name: payload.tool_name || currentSteps[existingIndex].tool_name,
                };
              } else if (sanitizedDescription) {
                currentSteps.push({
                  id: `step_${Date.now()}_${Math.random().toString(36).substring(2, 6)}`,
                  label: sanitizedDescription,
                  status: 'running',
                  integration_id: payload.integration_id,
                  tool_name: payload.tool_name,
                  timestamp: new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' }),
                });
              }
            }

            const newLogs = m.logs ? [...m.logs] : [];
            if (sanitizedDescription && !newLogs.some((l) => l.message === sanitizedDescription)) {
              newLogs.push({
                timestamp: new Date().toLocaleTimeString(),
                level: 'info' as const,
                message: sanitizedDescription,
              });
            }

            return {
              ...m,
              activityStatus:
                payload.status === 'done'
                  ? undefined
                  : {
                      status: payload.status,
                      description: sanitizedDescription,
                      integration_id: payload.integration_id,
                      tool_name: payload.tool_name,
                    },
              processSteps: currentSteps,
              process_steps: currentSteps,
              logs: newLogs,
            };
          }
          return m;
        });

        targetTask = {
          ...t,
          messages: updatedMessages,
        };
        return targetTask;
      }
      return t;
    });

    if (targetTask) {
      this.notify();
      this.scheduleDebouncedSave(targetTask);
    }
  }

  handleStreamDone(payload: StreamDonePayload): void {
    if (!payload || !payload.conversation_id || !payload.message_id) return;
    const isError = Boolean(payload.error);
    const nowTime = new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    this.cancelDebouncedSave(payload.conversation_id);

    const start = this.generationTimers.get(payload.message_id);
    let durationMs: number | undefined;
    let durationFormatted: string | undefined;
    if (start !== undefined) {
      durationMs = Math.round(performance.now() - start);
      this.generationTimers.delete(payload.message_id);
      durationFormatted = `${(durationMs / 1000).toFixed(1)}s`;
    }

    let targetTask: AgentTask | null = null;
    this.tasks = this.tasks.map((t) => {
      if (t.id === payload.conversation_id) {
        const updatedMessages = (t.messages || []).map((m) => {
          if (m.id === payload.message_id) {
            const finalStatus = isError ? ('error' as const) : ('done' as const);
            const errorLogs = payload.error
              ? [
                  ...(m.logs || []),
                  {
                    timestamp: new Date().toLocaleTimeString(),
                    level: 'error' as const,
                    message: payload.error,
                  },
                ]
              : m.logs;

            // Finalize process steps
            const finalizedSteps = (m.processSteps || m.process_steps || []).map((s) => ({
              ...s,
              status: s.status === 'running' ? (isError ? ('failed' as const) : ('completed' as const)) : s.status,
              error: (s.status === 'running' && isError ? payload.error : s.error) || undefined,
            }));

            // Only capture real provider usage and measured monotonic timing
            const metadata: MessageMetadata = {
              model: t.model,
              prompt_tokens: payload.prompt_tokens ?? undefined,
              completion_tokens: payload.completion_tokens ?? undefined,
              total_tokens: payload.total_tokens ?? undefined,
              duration_ms: durationMs,
              duration_formatted: durationFormatted,
            };

            const rawFinal = payload.full_text || m.text || '';
            return {
              ...m,
              text: sanitizeAssistantVisibleText(rawFinal),
              status: finalStatus,
              activityStatus: undefined,
              processSteps: finalizedSteps,
              process_steps: finalizedSteps,
              metadata,
              timestamp: nowTime,
              logs: errorLogs,
            };
          }
          return m;
        });

        const activeMsg = updatedMessages.find((m) => m.id === payload.message_id);
        const tokenUsage = payload.total_tokens
          ? {
              promptTokens: payload.prompt_tokens || 0,
              completionTokens: payload.completion_tokens || 0,
              totalTokens: payload.total_tokens,
            }
          : undefined;

        const updatedTask: AgentTask = {
          ...t,
          status: isError && !activeMsg?.text ? ('failed' as TaskStatus) : ('completed' as TaskStatus),
          completedAt: 'Just now',
          duration: durationFormatted || t.duration,
          output: activeMsg?.text || t.output,
          messages: updatedMessages,
          tokenUsage,
        };
        targetTask = updatedTask;
        return updatedTask;
      }
      return t;
    });


    this.notify();
    if (targetTask) {
      this.persistTask(targetTask);
    }
  }

  renameTask(id: string, newTitle: string): void {
    const trimmed = newTitle.trim();
    if (!trimmed) return;

    let targetTask: AgentTask | null = null;
    this.tasks = this.tasks.map((t) => {
      if (t.id === id) {
        targetTask = { ...t, title: trimmed };
        return targetTask;
      }
      return t;
    });

    this.notify();
    if (targetTask) {
      this.persistTask(targetTask);
      chatStorageService.renameConversation(id, trimmed).catch((err) => {
        console.error(`Failed to rename conversation [${id}]:`, err);
      });
    }
  }

  deleteTask(id: string): void {
    this.tasks = this.tasks.filter((t) => t.id !== id);
    if (this.activeTaskId === id) {
      this.setActiveTaskId(this.tasks[0]?.id || null);
    }
    this.notify();
    chatStorageService.deleteConversation(id).catch((err) => {
      console.error(`Failed to delete conversation [${id}]:`, err);
    });
  }

  clearAllTasks(): void {
    this.tasks = [];
    this.setActiveTaskId(null);
    this.notify();
    chatStorageService.clearAllConversations().catch((err) => {
      console.error('Failed to clear all conversations:', err);
    });
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
