import React from 'react';
import { ShieldCheck, GitBranch, Terminal, Cpu } from 'lucide-react';

export interface QuickActionChipsProps {
  onSelectPrompt: (prompt: string) => void;
}

export const QuickActionChips: React.FC<QuickActionChipsProps> = ({ onSelectPrompt }) => {
  const suggestions = [
    {
      icon: <ShieldCheck size={13} color="var(--accent-primary)" />,
      label: 'Security & Dependency Audit',
      prompt: 'Perform a security audit on Cargo dependencies and npm packages. Identify outdated libraries or vulnerable crates.',
    },
    {
      icon: <Terminal size={13} color="var(--accent-primary)" />,
      label: 'Inspect Rust Tauri IPC Commands',
      prompt: 'Check src-tauri/src/lib.rs for proper error handling and Windows Credential Manager integration.',
    },
    {
      icon: <GitBranch size={13} color="var(--accent-primary)" />,
      label: 'Analyze Uncommitted Git Diffs',
      prompt: 'Inspect current workspace git status, check unstaged changes, and summarize proposed modifications.',
    },
    {
      icon: <Cpu size={13} color="var(--accent-primary)" />,
      label: 'Generate TypeScript Types',
      prompt: 'Extract data contracts from backend models and generate comprehensive TypeScript definitions.',
    },
  ];

  return (
    <div
      style={{
        display: 'flex',
        flexWrap: 'wrap',
        gap: 'var(--space-2)',
        marginTop: 'var(--space-3)',
      }}
    >
      {suggestions.map((item, idx) => (
        <button
          key={idx}
          onClick={() => onSelectPrompt(item.prompt)}
          style={{
            display: 'inline-flex',
            alignItems: 'center',
            gap: '6px',
            padding: '5px 10px',
            backgroundColor: 'var(--bg-panel)',
            border: '1px solid var(--border-subtle)',
            borderRadius: 'var(--radius-full)',
            fontSize: '12px',
            color: 'var(--text-secondary)',
            cursor: 'pointer',
            transition: 'border-color var(--transition-fast), color var(--transition-fast)',
          }}
          onMouseEnter={(e) => {
            e.currentTarget.style.borderColor = 'var(--border-subtle-hover)';
            e.currentTarget.style.color = 'var(--text-primary)';
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.borderColor = 'var(--border-subtle)';
            e.currentTarget.style.color = 'var(--text-secondary)';
          }}
        >
          {item.icon}
          <span>{item.label}</span>
        </button>
      ))}
    </div>
  );
};
