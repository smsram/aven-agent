import { McpPlugin } from '../types';

const STORAGE_KEY = 'aven_mcp_plugins';

class PluginStore {
  private plugins: McpPlugin[] = [];
  private listeners: (() => void)[] = [];

  constructor() {
    this.loadFromStorage();
  }

  private loadFromStorage() {
    try {
      const saved = localStorage.getItem(STORAGE_KEY);
      if (saved) {
        this.plugins = JSON.parse(saved);
      } else {
        // Start with a clean empty state as requested: no fabricated connected plugins
        this.plugins = [];
      }
    } catch {
      this.plugins = [];
    }
  }

  private saveToStorage() {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(this.plugins));
    } catch {
      // ignore
    }
  }

  getPlugins(): McpPlugin[] {
    return [...this.plugins];
  }

  addPlugin(plugin: Omit<McpPlugin, 'id' | 'createdAt' | 'status'>): McpPlugin {
    const newPlugin: McpPlugin = {
      ...plugin,
      id: `mcp_${Date.now().toString(36)}`,
      createdAt: 'Just now',
      status: 'active',
    };
    this.plugins = [newPlugin, ...this.plugins];
    this.saveToStorage();
    this.notify();
    return newPlugin;
  }

  togglePlugin(id: string) {
    this.plugins = this.plugins.map((p) => {
      if (p.id === id) {
        return {
          ...p,
          status: p.status === 'active' ? 'disabled' : 'active',
        };
      }
      return p;
    });
    this.saveToStorage();
    this.notify();
  }

  deletePlugin(id: string) {
    this.plugins = this.plugins.filter((p) => p.id !== id);
    this.saveToStorage();
    this.notify();
  }

  subscribe(listener: () => void): () => void {
    this.listeners.push(listener);
    return () => {
      this.listeners = this.listeners.filter((l) => l !== listener);
    };
  }

  private notify() {
    this.listeners.forEach((l) => l());
  }
}

export const pluginStore = new PluginStore();
