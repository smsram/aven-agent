import React from 'react';

export interface InputProps extends React.InputHTMLAttributes<HTMLInputElement> {
  label?: string;
  helperText?: string;
  errorText?: string;
  prefixIcon?: React.ReactNode;
  suffixAction?: React.ReactNode;
}

export const Input = React.forwardRef<HTMLInputElement, InputProps>(({
  label,
  helperText,
  errorText,
  prefixIcon,
  suffixAction,
  style,
  className = '',
  disabled,
  ...props
}, ref) => {
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '6px', width: '100%' }}>
      {label && (
        <label
          style={{
            fontSize: '12px',
            fontWeight: 500,
            color: 'var(--text-secondary)',
            letterSpacing: '-0.01em',
          }}
        >
          {label}
        </label>
      )}
      <div
        style={{
          position: 'relative',
          display: 'flex',
          alignItems: 'center',
          width: '100%',
        }}
      >
        {prefixIcon && (
          <span
            style={{
              position: 'absolute',
              left: '10px',
              display: 'flex',
              alignItems: 'center',
              color: 'var(--text-muted)',
              pointerEvents: 'none',
            }}
          >
            {prefixIcon}
          </span>
        )}
        <input
          ref={ref}
          disabled={disabled}
          style={{
            width: '100%',
            height: '34px',
            backgroundColor: 'var(--bg-input)',
            border: `1px solid ${errorText ? 'var(--status-danger)' : 'var(--border-subtle)'}`,
            borderRadius: 'var(--radius-md)',
            color: 'var(--text-primary)',
            caretColor: 'var(--accent-primary)',
            fontSize: '13px',
            paddingLeft: prefixIcon ? '34px' : '10px',
            paddingRight: suffixAction ? '36px' : '10px',
            opacity: disabled ? 0.5 : 1,
            cursor: disabled ? 'not-allowed' : 'text',
            ...style,
          }}
          className={`input ${className}`}
          {...props}
        />
        {suffixAction && (
          <span
            style={{
              position: 'absolute',
              right: '8px',
              display: 'flex',
              alignItems: 'center',
            }}
          >
            {suffixAction}
          </span>
        )}
      </div>
      {errorText && (
        <span style={{ fontSize: '11px', color: 'var(--status-danger)' }}>{errorText}</span>
      )}
      {helperText && !errorText && (
        <span style={{ fontSize: '11px', color: 'var(--text-muted)' }}>{helperText}</span>
      )}
    </div>
  );
});

Input.displayName = 'Input';
