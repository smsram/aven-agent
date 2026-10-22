import React from 'react';
import { TaskInput } from './TaskInput';
import { RecentTasks } from './RecentTasks';
import { QuickActionChips } from './QuickActionChips';
import { ConversationView } from './ConversationView';
import { AgentTask, NavigationTab } from '../../types';

export interface HomePageProps {
  tasks: AgentTask[];
  activeTask?: AgentTask | null;
  onSelectTask: (task: AgentTask) => void;
  onSubmitTask: (prompt: string, model: string) => void;
  onSendReply: (text: string, model: string) => void;
  onNewChat: () => void;
  onNavigateTab: (tab: NavigationTab) => void;
  activeModel: string;
  onModelChange: (model: string) => void;
  availableModels: string[];
}

export const HomePage: React.FC<HomePageProps> = ({
  tasks,
  activeTask,
  onSelectTask,
  onSubmitTask,
  onSendReply,
  onNewChat,
  onNavigateTab,
  activeModel,
  onModelChange,
  availableModels,
}) => {
  // If a conversation is active, transition to properly styled ConversationView
  if (activeTask) {
    return (
      <ConversationView
        task={activeTask}
        onSendMessage={onSendReply}
        onNewChat={onNewChat}
        availableModels={availableModels}
      />
    );
  }

  // Otherwise, render initial clean New Chat screen
  return (
    <div
      style={{
        height: '100%',
        overflowY: 'auto',
        padding: '36px 24px',
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        backgroundColor: 'var(--bg-main)',
      }}
    >
      <div
        style={{
          width: '100%',
          maxWidth: '740px',
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
        }}
      >
        {/* Aven Hero Branding using public/aven.svg */}
        <div
          style={{
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            marginBottom: '24px',
            textAlign: 'center',
          }}
        >
          <img
            src="/aven.svg"
            alt="Aven Logo"
            style={{
              width: '42px',
              height: '42px',
              borderRadius: '8px',
              objectFit: 'contain',
              marginBottom: '12px',
            }}
          />

          <h1
            style={{
              fontSize: '20px',
              fontWeight: 600,
              letterSpacing: '-0.025em',
              color: 'var(--text-primary)',
              marginBottom: '4px',
            }}
          >
            Where should Aven assist you?
          </h1>
          <p
            style={{
              fontSize: '13px',
              color: 'var(--text-secondary)',
              maxWidth: '460px',
              lineHeight: 1.45,
            }}
          >
            Autonomous desktop engineering agent. Execute workspace inspections, refactor modules, and run terminal commands.
          </p>
        </div>

        {/* Central Task Input */}
        <TaskInput
          onSubmitTask={onSubmitTask}
          selectedModel={activeModel}
          onModelChange={onModelChange}
          availableModels={availableModels}
        />

        {/* Quick Action Suggestion Chips */}
        <QuickActionChips
          onSelectPrompt={(p) => onSubmitTask(p, activeModel)}
        />

        {/* Compact Recent Tasks List */}
        <RecentTasks
          tasks={tasks.slice(0, 4)}
          onSelectTask={onSelectTask}
          onViewAllActivity={() => onNavigateTab('activity')}
        />
      </div>
    </div>
  );
};
