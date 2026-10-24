import { Component, ErrorInfo, ReactNode } from 'react';
import { AlertCircle, RotateCcw } from 'lucide-react';

interface ErrorBoundaryProps {
  children: ReactNode;
  fallback?: ReactNode;
  onReset?: () => void;
  sectionName?: string;
}

interface ErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
}

/**
 * Resilient React ErrorBoundary that prevents unhandled render exceptions
 * from unmounting the whole application tree (which leaves only the background).
 */
export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  constructor(props: ErrorBoundaryProps) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo): void {
    console.error('ErrorBoundary caught an unhandled render error:', error, errorInfo);
  }

  handleReset = () => {
    this.setState({ hasError: false, error: null });
    if (this.props.onReset) {
      this.props.onReset();
    }
  };

  render(): ReactNode {
    if (this.state.hasError) {
      if (this.props.fallback) {
        return this.props.fallback;
      }

      const label = this.props.sectionName || 'Component';

      return (
        <div
          style={{
            padding: '12px 16px',
            margin: '8px 0',
            borderRadius: 'var(--radius-md, 8px)',
            backgroundColor: 'rgba(239, 68, 68, 0.08)',
            border: '1px solid rgba(239, 68, 68, 0.25)',
            color: 'var(--text-primary, #E2E8F0)',
            fontSize: '12px',
            display: 'flex',
            flexDirection: 'column',
            gap: '8px',
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <AlertCircle size={15} color="var(--status-danger, #EF4444)" style={{ flexShrink: 0 }} />
            <span style={{ fontWeight: 600, color: 'var(--status-danger, #EF4444)' }}>
              {label} encountered an unexpected display issue
            </span>
          </div>

          {this.state.error && (
            <div
              style={{
                fontFamily: 'var(--font-mono, monospace)',
                fontSize: '11px',
                color: 'var(--text-secondary, #94A3B8)',
                whiteSpace: 'pre-wrap',
                wordBreak: 'break-word',
                maxHeight: '80px',
                overflowY: 'auto',
                backgroundColor: 'rgba(0, 0, 0, 0.2)',
                padding: '6px 8px',
                borderRadius: '4px',
              }}
            >
              {this.state.error.message || String(this.state.error)}
            </div>
          )}

          <div style={{ display: 'flex', gap: '8px', marginTop: '2px' }}>
            <button
              type="button"
              onClick={this.handleReset}
              style={{
                display: 'inline-flex',
                alignItems: 'center',
                gap: '5px',
                padding: '4px 10px',
                borderRadius: 'var(--radius-sm, 6px)',
                backgroundColor: 'var(--bg-panel, #1E1F25)',
                border: '1px solid var(--border-subtle, #333)',
                color: 'var(--text-primary, #fff)',
                fontSize: '11px',
                cursor: 'pointer',
              }}
            >
              <RotateCcw size={11} color="var(--accent-primary, #7C7CF8)" />
              <span>Retry view</span>
            </button>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}
