import { useState, useEffect } from 'react';
import { NavigationTab, AgentTask, AppSettings, McpPlugin } from './types';
import { taskStore } from './services/taskStore';
import { pluginStore } from './services/pluginStore';
import { secureStore } from './services/tauriIpc';
import { Sidebar } from './components/layout/Sidebar';
import { TopBar } from './components/layout/TopBar';
import { HomePage } from './pages/Home/HomePage';
import { ActivityPage } from './pages/Activity/ActivityPage';
import { PluginsPage } from './pages/Plugins/PluginsPage';
import { SettingsPage } from './pages/Settings/SettingsPage';
import { CommandPaletteModal } from './components/common/CommandPaletteModal';

const DEFAULT_SETTINGS: AppSettings = {
  general: {
    defaultWorkspace: 'd:\\Projects\\Desktop\\Aven Agent',
    runInBackground: true,
    launchAtStartup: false,
    confirmDangerousCommands: true,
    telemetryEnabled: false,
  },
  appearance: {
    theme: 'dark-linear',
    compactDensity: false,
    fontSize: 'default',
    showStatusBar: true,
  },
  api: {
    activeProvider: 'cloudflare',
    activeModel: '@cf/qwen/qwen3.8-27b',
    temperature: 0.2,
    maxTokens: 4096,
  },
  permissions: {
    fileSystemAccess: 'read-write',
    shellExecution: 'auto-safe',
    networkAccess: 'full',
    restrictedPaths: ['.env', '.git/config', 'id_rsa'],
  },
  memory: {
    enableMemory: true,
    localVectorIndex: true,
    maxContextTokens: 64000,
    cacheSizeMb: 18.4,
  },
};

const AVAILABLE_MODELS = [
  '@cf/qwen/qwen3.8-27b',
];

export function App() {
  const [activeTab, setActiveTab] = useState<NavigationTab>('new-chat');
  const [activeConversationId, setActiveConversationId] = useState<string | null>(null);
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const [isSidebarCollapsed, setIsSidebarCollapsed] = useState(false);
  const [tasks, setTasks] = useState<AgentTask[]>(taskStore.getTasks());
  const [plugins, setPlugins] = useState<McpPlugin[]>(pluginStore.getPlugins());
  const [settings, setSettings] = useState<AppSettings>(DEFAULT_SETTINGS);
  const [showCommandPalette, setShowCommandPalette] = useState(false);

  // Subscribe to task store
  useEffect(() => {
    const unsubscribe = taskStore.subscribe(() => {
      setTasks(taskStore.getTasks());
    });
    return unsubscribe;
  }, []);

  // Subscribe to plugin store
  useEffect(() => {
    const unsubscribe = pluginStore.subscribe(() => {
      setPlugins(pluginStore.getPlugins());
    });
    return unsubscribe;
  }, []);

  // Load persistent settings from Tauri backend
  useEffect(() => {
    secureStore.loadSettings().then((loaded) => {
      if (loaded) {
        setSettings(loaded);
      }
    });
  }, []);

  // Global Keyboard Shortcuts
  useEffect(() => {
    const handleGlobalKeyDown = (e: KeyboardEvent) => {
      // Toggle Sidebar: Ctrl+B
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'b') {
        e.preventDefault();
        setIsSidebarCollapsed((prev) => !prev);
      }

      // Command Search Palette: Ctrl+K
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
        e.preventDefault();
        setShowCommandPalette((prev) => !prev);
      }

      // Quick Tab Nav: Ctrl+1..3
      if ((e.ctrlKey || e.metaKey) && !e.shiftKey) {
        if (e.key === '1') {
          e.preventDefault();
          setActiveConversationId(null);
          setActiveTab('new-chat');
        } else if (e.key === '2') {
          e.preventDefault();
          setActiveTab('activity');
        } else if (e.key === '3') {
          e.preventDefault();
          setActiveTab('plugins');
        } else if (e.key === ',') {
          e.preventDefault();
          setIsSettingsOpen((prev) => !prev);
        }
      }
    };

    window.addEventListener('keydown', handleGlobalKeyDown);
    return () => window.removeEventListener('keydown', handleGlobalKeyDown);
  }, []);

  const handleCreateTask = (prompt: string, model: string) => {
    const provider = 'Cloudflare Workers AI';
    const newTask = taskStore.createTask('', prompt, model, provider);
    setActiveConversationId(newTask.id);
    setActiveTab('new-chat');
  };

  const handleSendReply = (text: string, model: string) => {
    if (activeConversationId) {
      taskStore.appendUserMessage(activeConversationId, text, model);
    }
  };

  const handleSelectRecentTask = (task: AgentTask) => {
    setActiveConversationId(task.id);
    setActiveTab('new-chat');
  };

  const handleNewChat = () => {
    setActiveConversationId(null);
    setActiveTab('new-chat');
  };

  const activeTask = activeConversationId
    ? tasks.find((t) => t.id === activeConversationId) || null
    : null;

  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        width: '100vw',
        height: '100vh',
        overflow: 'hidden',
        backgroundColor: 'var(--bg-main)',
      }}
    >
      {/* Unified Window Title Bar and Top Navigation */}
      <TopBar
        onOpenSearch={() => setShowCommandPalette(true)}
      />

      {/* Main Container below Title Bar */}
      <div
        style={{
          display: 'flex',
          flex: 1,
          height: 'calc(100vh - 38px)',
          overflow: 'hidden',
          position: 'relative',
        }}
      >
        {/* Minimal Sidebar: New Chat, Activity, Plugins, Collapse at bottom, Settings */}
        <Sidebar
          activeTab={activeTab}
          onTabChange={(tab) => {
            setActiveTab(tab);
          }}
          onNewChat={handleNewChat}
          onOpenSettings={() => setIsSettingsOpen(true)}
          isSettingsOpen={isSettingsOpen}
          isCollapsed={isSidebarCollapsed}
          onToggleCollapse={() => setIsSidebarCollapsed(!isSidebarCollapsed)}
          taskCount={tasks.length}
        />

        {/* Workspace Viewport */}
        <main style={{ flex: 1, height: '100%', overflow: 'hidden', position: 'relative' }}>
          {activeTab === 'new-chat' ? (
            <HomePage
              tasks={tasks}
              activeTask={activeTask}
              onSelectTask={handleSelectRecentTask}
              onSubmitTask={handleCreateTask}
              onSendReply={handleSendReply}
              onNewChat={handleNewChat}
              onNavigateTab={(tab) => {
                setActiveTab(tab);
              }}
              activeModel={settings.api.activeModel}
              onModelChange={(model) => {
                const updated = {
                  ...settings,
                  api: { ...settings.api, activeModel: model },
                };
                setSettings(updated);
                secureStore.saveSettings(updated);
              }}
              availableModels={AVAILABLE_MODELS}
            />
          ) : activeTab === 'activity' ? (
            <ActivityPage
              tasks={tasks}
              onClearTasks={() => taskStore.clearAllTasks()}
              onNavigateNewChat={handleNewChat}
              onSelectTask={handleSelectRecentTask}
            />
          ) : (
            <PluginsPage
              plugins={plugins}
              onAddPlugin={(plugin) => pluginStore.addPlugin(plugin)}
              onTogglePlugin={(id) => pluginStore.togglePlugin(id)}
              onDeletePlugin={(id) => pluginStore.deletePlugin(id)}
            />
          )}
        </main>
      </div>

      {/* Settings Popup Modal: Centered within existing application window */}
      <SettingsPage
        isOpen={isSettingsOpen}
        settings={settings}
        onUpdateSettings={(newSettings) => setSettings(newSettings)}
        onClose={() => setIsSettingsOpen(false)}
      />

      {/* Quick Command Palette (Ctrl+K) */}
      <CommandPaletteModal
        isOpen={showCommandPalette}
        onClose={() => setShowCommandPalette(false)}
        onNavigateTab={(tab) => {
          if (tab === 'new-chat') {
            setActiveConversationId(null);
          }
          setActiveTab(tab);
        }}
        onOpenSettings={() => {
          setIsSettingsOpen(true);
        }}
        tasks={tasks}
        onSelectTask={handleSelectRecentTask}
        onSubmitQuickPrompt={(prompt) => handleCreateTask(prompt, settings.api.activeModel)}
      />
    </div>
  );
}

export default App;
