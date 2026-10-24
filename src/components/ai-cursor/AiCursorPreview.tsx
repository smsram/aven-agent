import React from 'react';
import { Loader2 } from 'lucide-react';

interface AiCursorPreviewProps {
  active?: boolean;
  onClose?: () => void;
  statusMessage?: string;
}

/**
 * Live Activity Indicator component.
 * Displays an unobtrusive live operation status during desktop automation.
 * Custom SVG cursor and pointer replacements have been completely removed.
 */
export const AiCursorPreview: React.FC<AiCursorPreviewProps> = ({
  active = false,
  statusMessage = 'Analyzing screen',
}) => {
  if (!active) return null;

  return (
    <div
      style={{
        position: 'fixed',
        top: '16px',
        left: '50%',
        transform: 'translateX(-50%)',
        backgroundColor: '#141720',
        border: '1px solid #686DF6',
        borderRadius: '8px',
        padding: '6px 14px',
        display: 'flex',
        alignItems: 'center',
        gap: '8px',
        zIndex: 9999,
        pointerEvents: 'none',
        boxShadow: '0 4px 16px rgba(0,0,0,0.4)',
      }}
    >
      <Loader2 size={14} className="animate-spin" style={{ color: '#686DF6' }} />
      <span
        style={{
          fontSize: '12px',
          fontWeight: 500,
          color: '#F0F0FF',
          letterSpacing: '0.01em',
        }}
      >
        {statusMessage}
      </span>
    </div>
  );
};

export default AiCursorPreview;
