import { AgentTask, ChatMessage, TaskStatus } from '../types';
import {
  cloudflareService,
  ChatMessagePayload,
  StreamChunkPayload,
  StreamDonePayload,
  isTauriDesktop,
} from './tauriIpc';
import { listen } from '@tauri-apps/api/event';

const SYSTEM_PROMPT =
  'You are Aven, a sophisticated engineering AI assistant. Provide concise, helpful, and well-structured answers using clean Markdown formatting.';

class TaskStore {
  private tasks: AgentTask[] = [];
  private listeners: (() => void)[] = [];
  private isListening = false;

  constructor() {
    this.initEventListeners();
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

  createTask(title: string, prompt: string, model: string, provider: string): AgentTask {
    const id = `tsk_${Date.now()}_${Math.random().toString(36).substring(2, 7)}`;
    const nowTime = new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    const userMsgId = `msg_u_${Date.now()}`;
    const asstMsgId = `msg_a_${Date.now()}`;

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
      tokenUsage: {
        promptTokens: Math.floor(prompt.length * 0.75),
        completionTokens: 0,
        totalTokens: Math.floor(prompt.length * 0.75),
      },
      messages: [userMessage, assistantMessage],
    };

    this.tasks = [newTask, ...this.tasks];
    this.notify();

    // Start streaming request to Cloudflare Workers AI
    const messagesPayload: ChatMessagePayload[] = [
      { role: 'system', content: SYSTEM_PROMPT },
      { role: 'user', content: prompt },
    ];

    cloudflareService.startChatStream(id, asstMsgId, messagesPayload).catch((err) => {
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
        {
          timestamp: new Date().toLocaleTimeString(),
          level: 'info',
          message: `Sending message to ${model}`,
        },
      ],
    };

    // Build multi-turn context (last 20 messages to keep bounds safe)
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

    this.tasks = this.tasks.map((t) => {
      if (t.id === taskId) {
        return {
          ...t,
          status: 'running',
          model,
          messages: [...(t.messages || []), userMsg, asstMsg],
        };
      }
      return t;
    });
    this.notify();

    cloudflareService.startChatStream(taskId, asstMsgId, historyPayload).catch((err) => {
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

    // Build context up to this assistant message
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

    // Reset this assistant message to thinking
    this.tasks = this.tasks.map((t) => {
      if (t.id === taskId) {
        const updated = (t.messages || []).map((m) => {
          if (m.id === assistantMsgId) {
            return {
              ...m,
              text: '',
              status: 'thinking' as const,
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
        return { ...t, status: 'running' as TaskStatus, messages: updated };
      }
      return t;
    });
    this.notify();

    cloudflareService.startChatStream(taskId, assistantMsgId, historyPayload).catch((err) => {
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

    this.tasks = this.tasks.map((t) => {
      if (t.id === taskId) {
        const updated = (t.messages || []).map((m) => {
          if (m.status === 'thinking') {
            return {
              ...m,
              status: 'done' as const,
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

        return {
          ...t,
          status: 'completed' as TaskStatus,
          messages: updated,
        };
      }
      return t;
    });
    this.notify();
  }

  handleStreamChunk(payload: StreamChunkPayload): void {
    let hasChanged = false;

    this.tasks = this.tasks.map((t) => {
      if (t.id === payload.conversation_id) {
        const updatedMessages = (t.messages || []).map((m) => {
          if (m.id === payload.message_id) {
            hasChanged = true;
            return {
              ...m,
              text: m.text + payload.chunk,
              status: 'thinking' as const,
            };
          }
          return m;
        });

        return {
          ...t,
          output: updatedMessages[updatedMessages.length - 1]?.text || t.output,
          messages: updatedMessages,
        };
      }
      return t;
    });

    if (hasChanged) {
      this.notify();
    }
  }

  handleStreamDone(payload: StreamDonePayload): void {
    const isError = Boolean(payload.error);
    const nowTime = new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });

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

            return {
              ...m,
              text: payload.full_text || m.text,
              status: finalStatus,
              timestamp: nowTime,
              logs: errorLogs,
            };
          }
          return m;
        });

        const activeMsg = updatedMessages.find((m) => m.id === payload.message_id);
        const completionLength = (activeMsg?.text || '').length;

        const promptTokens =
          payload.prompt_tokens ?? t.tokenUsage?.promptTokens ?? Math.max(1, Math.floor((t.prompt || '').length * 0.75));
        const completionTokens =
          payload.completion_tokens ?? Math.max(1, Math.floor(completionLength * 0.75));
        const totalTokens = payload.total_tokens ?? (promptTokens + completionTokens);

        return {
          ...t,
          status: isError && !activeMsg?.text ? ('failed' as TaskStatus) : ('completed' as TaskStatus),
          completedAt: 'Just now',
          output: activeMsg?.text || t.output,
          messages: updatedMessages,
          tokenUsage: {
            promptTokens,
            completionTokens,
            totalTokens,
          },
        };
      }
      return t;
    });

    this.notify();
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
