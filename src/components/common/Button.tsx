import React from 'react';

export interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'ghost' | 'danger' | 'outline';
  size?: 'sm' | 'md' | 'lg';
  icon?: React.ReactNode;
  iconPosition?: 'left' | 'right';
  shortcutHint?: string;
}

export const Button: React.FC<ButtonProps> = ({
  children,
  variant = 'secondary',
  size = 'md',
  icon,
  iconPosition = 'left',
  shortcutHint,
  className = '',
  disabled,
  style,
  ...props
}) => {
  const getVariantStyles = (): React.CSSProperties => {
    switch (variant) {
      case 'primary':
        return {
          backgroundColor: 'var(--accent-primary)',
          color: '#FFFFFF',
          border: '1px solid transparent',
          fontWeight: 500,
        };
      case 'ghost':
        return {
          backgroundColor: 'transparent',
          color: 'var(--text-secondary)',
          border: '1px solid transparent',
        };
      case 'danger':
        return {
          backgroundColor: 'var(--status-danger-bg)',
          color: 'var(--status-danger)',
          border: '1px solid var(--status-danger-border)',
        };
      case 'outline':
        return {
          backgroundColor: 'transparent',
          color: 'var(--text-primary)',
          border: '1px solid var(--border-subtle)',
        };
      case 'secondary':
      default:
        return {
          backgroundColor: 'var(--bg-panel)',
          color: 'var(--text-primary)',
          border: '1px solid var(--border-subtle)',
        };
    }
  };

  const getSizeStyles = (): React.CSSProperties => {
    switch (size) {
      case 'sm':
        return {
          padding: '4px 8px',
          fontSize: '12px',
          height: '28px',
          borderRadius: 'var(--radius-md)',
        };
      case 'lg':
        return {
          padding: '8px 16px',
          fontSize: '14px',
          height: '38px',
          borderRadius: 'var(--radius-md)',
        };
      case 'md':
      default:
        return {
          padding: '6px 12px',
          fontSize: '13px',
          height: '32px',
          borderRadius: 'var(--radius-md)',
        };
    }
  };

  return (
    <button
      disabled={disabled}
      style={{
        ...getVariantStyles(),
        ...getSizeStyles(),
        opacity: disabled ? 0.45 : 1,
        cursor: disabled ? 'not-allowed' : 'pointer',
        whiteSpace: 'nowrap',
        ...style,
      }}
      className={`btn btn-${variant} ${className}`}
      {...props}
    >
      {icon && iconPosition === 'left' && <span style={{ display: 'inline-flex' }}>{icon}</span>}
      {children}
      {icon && iconPosition === 'right' && <span style={{ display: 'inline-flex' }}>{icon}</span>}
      {shortcutHint && (
        <kbd
          style={{
            marginLeft: '6px',
            padding: '1px 5px',
            fontSize: '10px',
            borderRadius: 'var(--radius-sm)',
            backgroundColor: variant === 'primary' ? 'rgba(0,0,0,0.25)' : 'var(--bg-input)',
            color: variant === 'primary' ? '#FFFFFF' : 'var(--text-muted)',
            border: '1px solid var(--border-subtle)',
            fontFamily: 'var(--font-mono)',
          }}
        >
          {shortcutHint}
        </kbd>
      )}
    </button>
  );
};
