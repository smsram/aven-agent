import React from 'react';

export interface BadgeProps {
  children: React.ReactNode;
  variant?: 'default' | 'indigo' | 'success' | 'warning' | 'danger' | 'muted';
  dot?: boolean;
  size?: 'sm' | 'md';
  style?: React.CSSProperties;
}

export const Badge: React.FC<BadgeProps> = ({
  children,
  variant = 'default',
  dot = false,
  size = 'md',
  style,
}) => {
  const getColors = () => {
    switch (variant) {
      case 'indigo':
        return {
          bg: 'var(--accent-subtle)',
          border: 'var(--accent-border)',
          text: 'var(--text-accent)',
          dot: 'var(--accent-primary)',
        };
      case 'success':
        return {
          bg: 'var(--status-success-bg)',
          border: 'var(--status-success-border)',
          text: 'var(--status-success)',
          dot: 'var(--status-success)',
        };
      case 'warning':
        return {
          bg: 'var(--status-warning-bg)',
          border: 'var(--status-warning-border)',
          text: 'var(--status-warning)',
          dot: 'var(--status-warning)',
        };
      case 'danger':
        return {
          bg: 'var(--status-danger-bg)',
          border: 'var(--status-danger-border)',
          text: 'var(--status-danger)',
          dot: 'var(--status-danger)',
        };
      case 'muted':
        return {
          bg: 'rgba(110, 112, 123, 0.1)',
          border: 'var(--border-subtle)',
          text: 'var(--text-muted)',
          dot: 'var(--text-muted)',
        };
      case 'default':
      default:
        return {
          bg: 'var(--bg-panel)',
          border: 'var(--border-subtle)',
          text: 'var(--text-secondary)',
          dot: 'var(--text-secondary)',
        };
    }
  };

  const colors = getColors();
  const isSm = size === 'sm';

  return (
    <span
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: '5px',
        padding: isSm ? '1px 6px' : '2px 8px',
        fontSize: isSm ? '11px' : '12px',
        borderRadius: 'var(--radius-sm)',
        backgroundColor: colors.bg,
        border: `1px solid ${colors.border}`,
        color: colors.text,
        fontWeight: 500,
        lineHeight: 1.3,
        letterSpacing: '-0.01em',
        ...style,
      }}
    >
      {dot && (
        <span
          style={{
            width: '6px',
            height: '6px',
            borderRadius: '50%',
            backgroundColor: colors.dot,
            display: 'inline-block',
          }}
        />
      )}
      {children}
    </span>
  );
};
