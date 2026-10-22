import React from 'react';

export interface CardProps extends React.HTMLAttributes<HTMLDivElement> {
  children: React.ReactNode;
  interactive?: boolean;
  padded?: boolean;
  compact?: boolean;
}

export const Card: React.FC<CardProps> = ({
  children,
  interactive = false,
  padded = true,
  compact = false,
  style,
  className = '',
  ...props
}) => {
  return (
    <div
      style={{
        backgroundColor: 'var(--bg-panel)',
        border: '1px solid var(--border-subtle)',
        borderRadius: 'var(--radius-lg)',
        padding: padded ? (compact ? 'var(--space-3)' : 'var(--space-4)') : '0',
        transition: 'border-color var(--transition-fast), background-color var(--transition-fast)',
        cursor: interactive ? 'pointer' : 'default',
        ...style,
      }}
      className={`card ${interactive ? 'card-interactive' : ''} ${className}`}
      onMouseEnter={(e) => {
        if (interactive) {
          e.currentTarget.style.borderColor = 'var(--border-subtle-hover)';
          e.currentTarget.style.backgroundColor = 'var(--bg-panel-hover)';
        }
      }}
      onMouseLeave={(e) => {
        if (interactive) {
          e.currentTarget.style.borderColor = (style?.borderColor as string) || 'var(--border-subtle)';
          e.currentTarget.style.backgroundColor = (style?.backgroundColor as string) || 'var(--bg-panel)';
        }
      }}
      {...props}
    >
      {children}
    </div>
  );
};
