import React from 'react';

export interface SwitchProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
  label?: string;
  description?: string;
}

export const Switch: React.FC<SwitchProps> = ({
  checked,
  onChange,
  disabled = false,
  label,
  description,
}) => {
  return (
    <div
      onClick={() => {
        if (!disabled) onChange(!checked);
      }}
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        cursor: disabled ? 'not-allowed' : 'pointer',
        opacity: disabled ? 0.5 : 1,
        gap: 'var(--space-4)',
        userSelect: 'none',
        textAlign: 'left',
      }}
    >
      {(label || description) && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '2px', textAlign: 'left', flex: 1 }}>
          {label && (
            <span style={{ fontSize: '13px', fontWeight: 500, color: 'var(--text-primary)', textAlign: 'left' }}>
              {label}
            </span>
          )}
          {description && (
            <span style={{ fontSize: '12px', color: 'var(--text-secondary)', textAlign: 'left' }}>
              {description}
            </span>
          )}
        </div>
      )}
      <button
        type="button"
        role="switch"
        aria-checked={checked}
        disabled={disabled}
        onClick={(e) => {
          e.stopPropagation();
          if (!disabled) onChange(!checked);
        }}
        style={{
          width: '36px',
          height: '20px',
          borderRadius: '9999px',
          backgroundColor: checked ? 'var(--accent-primary)' : '#34353C',
          position: 'relative',
          padding: '2px',
          border: 'none',
          outline: 'none',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'flex-start',
          transition: 'background-color var(--transition-fast)',
          cursor: disabled ? 'not-allowed' : 'pointer',
          flexShrink: 0,
        }}
      >
        <span
          style={{
            display: 'block',
            width: '16px',
            height: '16px',
            borderRadius: '50%',
            backgroundColor: '#FFFFFF',
            transform: checked ? 'translateX(16px)' : 'translateX(0)',
            transition: 'transform var(--transition-fast)',
            boxShadow: '0 1px 3px rgba(0, 0, 0, 0.4)',
          }}
        />
      </button>
    </div>
  );
};
