import {
  CatalogEntry,
  PluginMetadata,
  PluginConfig,
  RegisteredCapability,
  TestConnectionResult,
  CapabilityExecutionRequest,
  CapabilityExecutionResult,
} from '../types';
import { pluginsService, isTauriDesktop } from './tauriIpc';
import { listen } from '@tauri-apps/api/event';

class PluginStore {
  private catalog: CatalogEntry[] = [];
  private plugins: PluginMetadata[] = [];
  private capabilities: RegisteredCapability[] = [];
  private systemInfo: Record<string, unknown> = {};
  private loading = false;
  private error: string | null = null;
  private listeners: (() => void)[] = [];
  private isListening = false;

  constructor() {
    this.refresh();
    this.initTauriListener();
  }

  private async initTauriListener() {
    if (this.isListening || !isTauriDesktop()) return;
    this.isListening = true;
    try {
      await listen('plugins-state-changed', () => {
        this.refresh();
      });
    } catch (err) {
      console.error('Failed to listen for plugins-state-changed event:', err);
    }
  }

  async refresh(): Promise<void> {
    this.loading = true;
    try {
      const state = await pluginsService.getPluginsState();
      this.catalog = state.catalog || [];
      this.plugins = state.configured_plugins || [];
      this.capabilities = state.capabilities || [];
      this.systemInfo = state.system_info || {};
      this.error = null;
    } catch (err) {
      console.error('Failed to load plugins state from backend:', err);
      this.error = String(err);
    } finally {
      this.loading = false;
      this.notify();
    }
  }

  getCatalog(): CatalogEntry[] {
    return [...this.catalog];
  }

  getPlugins(): PluginMetadata[] {
    return [...this.plugins];
  }

  getPluginById(id: string): PluginMetadata | undefined {
    return this.plugins.find((p) => p.id === id);
  }

  getCatalogEntryById(id: string): CatalogEntry | undefined {
    return this.catalog.find((c) => c.id === id);
  }

  getCapabilities(): RegisteredCapability[] {
    return [...this.capabilities];
  }

  getSystemInfo(): Record<string, unknown> {
    return { ...this.systemInfo };
  }

  isLoading(): boolean {
    return this.loading;
  }

  getError(): string | null {
    return this.error;
  }

  async savePlugin(config: PluginConfig, secretKey?: string): Promise<PluginMetadata> {
    const updated = await pluginsService.savePluginConfig(config, secretKey);
    await this.refresh();
    return updated;
  }

  async deletePlugin(id: string): Promise<void> {
    await pluginsService.deletePluginConfig(id);
    await this.refresh();
  }

  async testConnection(id: string): Promise<TestConnectionResult> {
    return await pluginsService.testPluginConnection(id);
  }

  async connect(id: string): Promise<PluginMetadata> {
    const updated = await pluginsService.connectPlugin(id);
    await this.refresh();
    return updated;
  }

  async disconnect(id: string): Promise<PluginMetadata> {
    const updated = await pluginsService.disconnectPlugin(id);
    await this.refresh();
    return updated;
  }

  async executeCapability(
    request: CapabilityExecutionRequest
  ): Promise<CapabilityExecutionResult> {
    return await pluginsService.executeCapability(request);
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
