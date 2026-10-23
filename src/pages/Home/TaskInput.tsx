import React, { useState, useRef, useEffect } from 'react';
import { ArrowUp, Sparkles, ChevronDown, Check } from 'lucide-react';
import { Button } from '../../components/common/Button';

export interface TaskInputProps {
  onSubmitTask: (prompt: string, model: string) => void;
  selectedModel: string;
  onModelChange: (model: string) => void;
  availableModels: string[];
}

export const TaskInput: React.FC<TaskInputProps> = ({
  onSubmitTask,
  selectedModel,
  onModelChange,
  availableModels,
}) => {
  const [prompt, setPrompt] = useState('');
  const [showModelDropdown, setShowModelDropdown] = useState(false);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const dropdownRef = useRef<HTMLDivElement>(null);

  const handleSubmit = (e?: React.FormEvent) => {
    if (e) e.preventDefault();
    if (!prompt.trim()) return;
    onSubmitTask(prompt.trim(), selectedModel);
    setPrompt('');
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && (e.ctrlKey || e.metaKey || !e.shiftKey)) {
      e.preventDefault();
      handleSubmit();
    }
  };

  // Close model dropdown on outside click
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(e.target as Node)) {
        setShowModelDropdown(false);
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  return (
    <div
      style={{
        position: 'relative',
        width: '100%',
        backgroundColor: 'var(--bg-panel)',
        border: '1px solid var(--border-subtle)',
        borderRadius: 'var(--radius-xl)',
        padding: 'var(--space-3) var(--space-4)',
        transition: 'border-color var(--transition-fast)',
      }}
      onFocus={() => {
        const el = textareaRef.current?.parentElement;
        if (el) el.style.borderColor = 'var(--border-subtle-hover)';
      }}
    >
      {/* Central Task Textarea */}
      <textarea
        ref={textareaRef}
        value={prompt}
        onChange={(e) => setPrompt(e.target.value)}
        onKeyDown={handleKeyDown}
        placeholder="Ask Aven anything..."
        rows={3}
        aria-label="Ask Aven anything"
        style={{
          width: '100%',
          backgroundColor: 'transparent',
          border: 'none',
          outline: 'none',
          color: 'var(--text-primary)',
          fontSize: '14px',
          lineHeight: '1.5',
          resize: 'none',
          padding: '4px 0',
        }}
      />

      {/* Bottom control row: Model selector pill and Action Button */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          paddingTop: 'var(--space-2)',
          borderTop: '1px solid var(--border-subtle-dim)',
          marginTop: 'var(--space-2)',
        }}
      >
        {/* Model Picker Pill */}
        <div style={{ position: 'relative' }} ref={dropdownRef}>
          <button
            type="button"
            onClick={() => setShowModelDropdown(!showModelDropdown)}
            style={{
              display: 'inline-flex',
              alignItems: 'center',
              gap: '6px',
              padding: '4px 8px',
              borderRadius: 'var(--radius-sm)',
              backgroundColor: 'var(--bg-input)',
              border: '1px solid var(--border-subtle)',
              fontSize: '12px',
              color: 'var(--text-secondary)',
              fontFamily: 'var(--font-mono)',
            }}
          >
            <Sparkles size={12} color="var(--accent-primary)" />
            <span style={{ fontWeight: 500, color: 'var(--text-primary)' }}>{selectedModel}</span>
            <ChevronDown size={12} color="var(--text-muted)" />
          </button>

          {/* Dropdown Menu */}
          {showModelDropdown && (
            <div
              style={{
                position: 'absolute',
                top: '32px',
                left: 0,
                width: '240px',
                backgroundColor: 'var(--bg-dropdown)',
                border: '1px solid var(--border-subtle)',
                borderRadius: 'var(--radius-md)',
                padding: '4px',
                boxShadow: '0 8px 24px rgba(0,0,0,0.5)',
                zIndex: 50,
              }}
            >
              <div
                style={{
                  padding: '4px 8px',
                  fontSize: '11px',
                  color: 'var(--text-muted)',
                  textTransform: 'uppercase',
                  fontWeight: 600,
                  letterSpacing: '0.04em',
                }}
              >
                Cloudflare Workers AI
              </div>
              {availableModels.map((m) => {
                const isSelected = m === selectedModel;
                return (
                  <button
                    key={m}
                    onClick={() => {
                      onModelChange(m);
                      setShowModelDropdown(false);
                    }}
                    style={{
                      width: '100%',
                      padding: '6px 8px',
                      borderRadius: 'var(--radius-sm)',
                      backgroundColor: isSelected ? 'var(--accent-subtle)' : 'transparent',
                      color: isSelected ? 'var(--text-accent)' : 'var(--text-secondary)',
                      fontSize: '12px',
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'space-between',
                    }}
                    onMouseEnter={(e) => {
                      if (!isSelected) e.currentTarget.style.backgroundColor = 'var(--bg-panel-hover)';
                    }}
                    onMouseLeave={(e) => {
                      if (!isSelected) e.currentTarget.style.backgroundColor = 'transparent';
                    }}
                  >
                    <span>{m}</span>
                    {isSelected && <Check size={13} color="var(--accent-primary)" />}
                  </button>
                );
              })}
            </div>
          )}
        </div>

        {/* Primary Action Button */}
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          <Button
            variant="primary"
            size="md"
            onClick={() => handleSubmit()}
            disabled={!prompt.trim()}
            icon={<ArrowUp size={14} />}
            iconPosition="right"
          >
            Run Task
          </Button>
        </div>
      </div>

    </div>
  );
};
