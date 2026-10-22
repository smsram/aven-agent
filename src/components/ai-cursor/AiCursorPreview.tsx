import React, { useState, useEffect } from 'react';
import { Sparkles, X } from 'lucide-react';

export interface AiCursorPreviewProps {
  active: boolean;
  onClose: () => void;
}

export const AiCursorPreview: React.FC<AiCursorPreviewProps> = ({ active, onClose }) => {
  const [pos, setPos] = useState({ x: -100, y: -100 });
  const [statusMessage, setStatusMessage] = useState('Aven: Ready for instructions');

  const statusPresets = [
    'Aven: Ready for instructions',
    'Aven: Analyzing syntax tree...',
    'Aven: Cross-referencing Rust bindings...',
    'Aven: Generating typed response...',
  ];

  useEffect(() => {
    if (!active) return;

    const handleMouseMove = (e: MouseEvent) => {
      setPos({ x: e.clientX, y: e.clientY });
    };

    window.addEventListener('mousemove', handleMouseMove);
    return () => window.removeEventListener('mousemove', handleMouseMove);
  }, [active]);

  // Subtle status rotation
  useEffect(() => {
    if (!active) return;
    let idx = 0;
    const interval = setInterval(() => {
      idx = (idx + 1) % statusPresets.length;
      setStatusMessage(statusPresets[idx]);
    }, 4000);
    return () => clearInterval(interval);
  }, [active]);

  if (!active) return null;

  return (
    <>
      {/* Floating Control HUD in bottom corner */}
      <div
        style={{
          position: 'fixed',
          bottom: '20px',
          right: '20px',
          backgroundColor: 'var(--bg-panel)',
          border: '1px solid var(--accent-border)',
          borderRadius: 'var(--radius-lg)',
          padding: 'var(--space-3) var(--space-4)',
          boxShadow: '0 8px 32px rgba(0,0,0,0.4)',
          zIndex: 9999,
          display: 'flex',
          alignItems: 'center',
          gap: 'var(--space-3)',
          maxWidth: '380px',
        }}
      >
        <div
          style={{
            width: '28px',
            height: '28px',
            borderRadius: 'var(--radius-md)',
            backgroundColor: 'var(--accent-subtle)',
            color: 'var(--accent-primary)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            flexShrink: 0,
          }}
        >
          <Sparkles size={16} />
        </div>
        <div style={{ flex: 1 }}>
          <div style={{ fontSize: '12px', fontWeight: 600, color: 'var(--text-primary)' }}>
            AI Cursor Interaction Reserved
          </div>
          <div style={{ fontSize: '11px', color: 'var(--text-secondary)', marginTop: '2px' }}>
            Electric indigo cursor & floating status pill preview mode is active. Move mouse across workspace.
          </div>
        </div>
        <button
          onClick={onClose}
          style={{
            width: '24px',
            height: '24px',
            borderRadius: 'var(--radius-sm)',
            color: 'var(--text-muted)',
            padding: 0,
          }}
          title="Exit cursor preview"
        >
          <X size={14} />
        </button>
      </div>

      {/* Indigo Custom AI Cursor & Compact Floating Status Pill */}
      {pos.x > 0 && (
        <div
          style={{
            position: 'fixed',
            left: `${pos.x}px`,
            top: `${pos.y}px`,
            pointerEvents: 'none',
            zIndex: 9998,
            transform: 'translate(4px, 4px)',
            display: 'flex',
            alignItems: 'flex-start',
            gap: '8px',
          }}
        >
          {/* Custom Electric Indigo Cursor Icon */}
          <div
            style={{
              position: 'relative',
              width: '18px',
              height: '18px',
              filter: 'drop-shadow(0 2px 8px rgba(124, 124, 248, 0.6))',
            }}
          >
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none">
              <path
                d="M3 3l7.07 16.97 2.51-7.39 7.39-2.51L3 3z"
                fill="var(--accent-primary)"
                stroke="#FFFFFF"
                strokeWidth="1.5"
                strokeLinejoin="round"
              />
            </svg>
            {/* Indigo beacon ping */}
            <span
              style={{
                position: 'absolute',
                top: '0px',
                left: '0px',
                width: '6px',
                height: '6px',
                borderRadius: '50%',
                backgroundColor: 'var(--accent-primary)',
              }}
            />
          </div>

          {/* Compact Floating Status Message beside the cursor */}
          <div
            style={{
              backgroundColor: 'var(--bg-panel)',
              border: '1px solid var(--accent-border)',
              borderRadius: 'var(--radius-full)',
              padding: '3px 10px',
              boxShadow: '0 4px 14px rgba(0,0,0,0.3)',
              display: 'flex',
              alignItems: 'center',
              gap: '6px',
              whiteSpace: 'nowrap',
            }}
          >
            <span
              style={{
                width: '6px',
                height: '6px',
                borderRadius: '50%',
                backgroundColor: 'var(--accent-primary)',
                boxShadow: '0 0 6px var(--accent-primary)',
              }}
            />
            <span
              style={{
                fontSize: '11px',
                fontWeight: 500,
                color: 'var(--text-primary)',
                fontFamily: 'var(--font-mono)',
              }}
            >
              {statusMessage}
            </span>
          </div>
        </div>
      )}
    </>
  );
};
