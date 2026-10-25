import React, { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { Loader2, CheckCircle2, AlertCircle, XCircle } from 'lucide-react';
import { sanitizeProgressDescription } from '../../services/taskStore';

interface AutomationSessionPayload {
  session_id: string;
  state: 'idle' | 'running' | 'waiting_for_model' | 'executing_tool' | 'verifying' | 'recovering' | 'completed' | 'failed' | 'cancelled';
  label: string;
  detail?: string | null;
  target?: string | null;
  pointer_x?: number | null;
  pointer_y?: number | null;
}

interface ChatActivityStatusPayload {
  status: string;
  description: string;
  tool_name?: string | null;
}

export const ActivityOverlayWindow: React.FC = () => {
  const [session, setSession] = useState<AutomationSessionPayload>({
    session_id: '',
    state: 'running',
    label: 'Analyzing screen',
  });
  const [visible, setVisible] = useState(true);

  useEffect(() => {
    let unlistenSession: (() => void) | undefined;
    let unlistenChat: (() => void) | undefined;

    const setupListeners = async () => {
      try {
        const unSession = await listen<AutomationSessionPayload>('automation-session-state', (event) => {
          const payload = event.payload;
          setSession({
            ...payload,
            label: sanitizeProgressDescription(payload.label),
          });
          if (payload.state === 'idle') {
            setVisible(false);
          } else {
            setVisible(true);
          }
        });
        unlistenSession = unSession;

        const unChat = await listen<ChatActivityStatusPayload>('chat-activity-status', (event) => {
          const { status, description } = event.payload;
          const cleanDesc = sanitizeProgressDescription(description);
          if (status === 'completed' || status === 'failed' || status === 'cancelled') {
            setSession(prev => ({
              ...prev,
              state: status as AutomationSessionPayload['state'],
              label: cleanDesc || (status.charAt(0).toUpperCase() + status.slice(1)),
            }));
          } else if (cleanDesc) {
            setSession(prev => ({
              ...prev,
              label: cleanDesc,
            }));
          }
        });
        unlistenChat = unChat;
      } catch (err) {
        console.error('Failed to register overlay event listeners:', err);
      }
    };

    setupListeners();

    return () => {
      if (unlistenSession) unlistenSession();
      if (unlistenChat) unlistenChat();
    };
  }, []);

  if (!visible) return null;

  const isTerminal = session.state === 'completed' || session.state === 'failed' || session.state === 'cancelled';
  const isSuccess = session.state === 'completed';
  const isFailed = session.state === 'failed';
  const isCancelled = session.state === 'cancelled';

  return (
    <div className="activity-overlay-viewport">
      <div className={`activity-overlay-badge ${session.state}`}>
        <div className="activity-overlay-icon-wrap">
          {isSuccess ? (
            <CheckCircle2 className="activity-overlay-status-icon text-success" size={15} />
          ) : isFailed ? (
            <AlertCircle className="activity-overlay-status-icon text-danger" size={15} />
          ) : isCancelled ? (
            <XCircle className="activity-overlay-status-icon text-warning" size={15} />
          ) : (
            <Loader2 className="activity-overlay-spinner text-accent" size={15} />
          )}
        </div>

        <span className="activity-overlay-label">
          {session.label || 'Analyzing screen'}
        </span>

        {session.target && !isTerminal && (
          <span className="activity-overlay-target">
            • {session.target}
          </span>
        )}
      </div>
    </div>
  );
};
