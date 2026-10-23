import {
  Folder,
  Server,
  Globe,
  Layers,
  Plug,
} from 'lucide-react';

export interface IntegrationIconProps {
  id?: string;
  integrationId?: string;
  name?: string;
  type?: 'mcp' | 'api' | 'native' | string;
  size?: number;
  className?: string;
  style?: React.CSSProperties;
  title?: string;
}


/** Official GitHub Mark SVG */
export const GitHubLogo: React.FC<{ size?: number; className?: string; style?: React.CSSProperties }> = ({
  size = 16,
  className = '',
  style,
}) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill="currentColor"
    className={`inline-block shrink-0 ${className}`}
    style={{ color: '#F4F4F5', ...style }}
    role="img"
    aria-label="GitHub"
  >
    <path
      fillRule="evenodd"
      clipRule="evenodd"
      d="M12 2C6.477 2 2 6.484 2 12.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.53 1.032 1.53 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0112 6.844c.85.004 1.705.115 2.504.337 1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10.019 0 0022 12.017C22 6.484 17.522 2 12 2z"
    />
  </svg>
);

/** Google / Gemini Official Multicolored Vector Logo */
export const GoogleGeminiLogo: React.FC<{ size?: number; className?: string; style?: React.CSSProperties }> = ({
  size = 16,
  className = '',
  style,
}) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 24 24"
    className={`inline-block shrink-0 ${className}`}
    style={style}
    role="img"
    aria-label="Google Gemini Search Grounding"
  >
    <path
      fill="#4285F4"
      d="M23.745 12.27c0-.7-.06-1.4-.19-2.07H12v4.51h6.6c-.29 1.52-1.14 2.82-2.4 3.68v3.05h3.88c2.27-2.09 3.665-5.17 3.665-9.17z"
    />
    <path
      fill="#34A853"
      d="M12 24c3.24 0 5.95-1.08 7.93-2.91l-3.88-3.05c-1.08.72-2.45 1.16-4.05 1.16-3.12 0-5.77-2.1-6.72-4.93H1.24v3.15C3.26 21.36 7.33 24 12 24z"
    />
    <path
      fill="#FBBC05"
      d="M5.28 14.27c-.25-.72-.38-1.49-.38-2.27s.13-1.55.38-2.27V6.58H1.24C.45 8.16 0 9.94 0 12s.45 3.84 1.24 5.42l4.04-3.15z"
    />
    <path
      fill="#EA4335"
      d="M12 4.75c1.77 0 3.35.61 4.6 1.8l3.42-3.42C17.95 1.19 15.24 0 12 0 7.33 0 3.26 2.64 1.24 6.58l4.04 3.15c.95-2.83 3.6-4.98 6.72-4.98z"
    />
  </svg>
);

/** Official Playwright Theater Masks Vector Logo */
export const PlaywrightLogo: React.FC<{ size?: number; className?: string; style?: React.CSSProperties }> = ({
  size = 16,
  className = '',
  style,
}) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill="none"
    className={`inline-block shrink-0 ${className}`}
    style={style}
    role="img"
    aria-label="Playwright"
  >
    {/* Mask 1 (Green) */}
    <path
      d="M14.5 4.5C10.5 4.5 7.5 7.5 7.5 11.5C7.5 15.5 10.5 18.5 14.5 18.5C18.5 18.5 21.5 15.5 21.5 11.5C21.5 7.5 18.5 4.5 14.5 4.5Z"
      stroke="#2EAD33"
      strokeWidth="2"
      fill="#2EAD33"
      fillOpacity="0.25"
    />
    <circle cx="12.5" cy="10" r="1.2" fill="#2EAD33" />
    <circle cx="16.5" cy="10" r="1.2" fill="#2EAD33" />
    <path d="M12.5 14C13.5 15 15.5 15 16.5 14" stroke="#2EAD33" strokeWidth="1.5" strokeLinecap="round" />

    {/* Mask 2 (Red/Orange outline) */}
    <path
      d="M9.5 7.5C5.5 7.5 2.5 10.5 2.5 14.5C2.5 18.5 5.5 21.5 9.5 21.5C13.5 21.5 16.5 18.5 16.5 14.5"
      stroke="#E05D44"
      strokeWidth="2"
      strokeLinecap="round"
    />
    <circle cx="6.5" cy="13" r="1" fill="#E05D44" />
    <path d="M6 17C7 16.2 8.5 16.2 9.5 17" stroke="#E05D44" strokeWidth="1.2" strokeLinecap="round" />
  </svg>
);

/** SQLite Feather & Cylinder Vector Logo */
export const SQLiteLogo: React.FC<{ size?: number; className?: string; style?: React.CSSProperties }> = ({
  size = 16,
  className = '',
  style,
}) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill="none"
    className={`inline-block shrink-0 ${className}`}
    style={style}
    role="img"
    aria-label="SQLite"
  >
    {/* Stylized SQLite Feather / DB Hybrid */}
    <path
      d="M4 6C4 4 7.58 2.5 12 2.5C16.42 2.5 20 4 20 6V18C20 20 16.42 21.5 12 21.5C7.58 21.5 4 20 4 18V6Z"
      fill="#003B57"
      fillOpacity="0.2"
      stroke="#003B57"
      strokeWidth="1.5"
    />
    <ellipse cx="12" cy="6" rx="8" ry="3.5" fill="#003B57" fillOpacity="0.4" stroke="#38BDF8" strokeWidth="1.5" />
    <path d="M4 12C4 13.93 7.58 15.5 12 15.5C16.42 15.5 20 13.93 20 12" stroke="#38BDF8" strokeWidth="1.5" />
    {/* Feather quill accent */}
    <path
      d="M15 7L18.5 3.5C19 3 20 3 20.5 3.5C21 4 21 5 20.5 5.5L16.5 9.5"
      stroke="#38BDF8"
      strokeWidth="1.5"
      strokeLinecap="round"
    />
  </svg>
);

/** Official Microsoft Windows 4-Pane Modern Logo */
export const WindowsLogo: React.FC<{ size?: number; className?: string; style?: React.CSSProperties }> = ({
  size = 16,
  className = '',
  style,
}) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill="currentColor"
    className={`inline-block shrink-0 ${className}`}
    style={{ color: '#00ADEF', ...style }}
    role="img"
    aria-label="Windows"
  >
    <path d="M2.5 3.5H11V11.5H2.5V3.5ZM12.5 3.5H21.5V11.5H12.5V3.5ZM2.5 12.5H11V20.5H2.5V12.5ZM12.5 12.5H21.5V20.5H12.5V12.5Z" />
  </svg>
);

/** Context7 Code Intelligence Vector Logo */
export const Context7Logo: React.FC<{ size?: number; className?: string; style?: React.CSSProperties }> = ({
  size = 16,
  className = '',
  style,
}) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill="none"
    className={`inline-block shrink-0 ${className}`}
    style={style}
    role="img"
    aria-label="Context7"
  >
    <polygon points="12,2 21,7 21,17 12,22 3,17 3,7" stroke="#7C7CF8" strokeWidth="1.75" fill="rgba(124, 124, 248, 0.2)" />
    <path d="M9 8H15L11 16H10" stroke="#7C7CF8" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
  </svg>
);

/**
 * Centralized, platform-specific integration icon resolver.
 * Replaces generic icons with accurate platform marks (GitHub, Google, SQLite, Playwright, Windows, etc.)
 */
export const IntegrationIcon: React.FC<IntegrationIconProps> = ({
  id = '',
  integrationId = '',
  name = '',
  type = '',
  size = 16,
  className = '',
  style,
  title,
}) => {
  const actualId = id || integrationId || '';
  const normId = actualId.toLowerCase().replace(/::/g, '_').replace(/:/g, '_').replace(/-/g, '_');
  const normName = name.toLowerCase();


  // 1. GitHub
  if (
    normId.includes('github') ||
    normName.includes('github')
  ) {
    return <GitHubLogo size={size} className={className} style={style} />;
  }

  // 2. Google Search / Gemini
  if (
    normId.includes('gemini') ||
    normId.includes('google_search') ||
    normName.includes('google') ||
    normName.includes('gemini')
  ) {
    return <GoogleGeminiLogo size={size} className={className} style={style} />;
  }

  // 3. Playwright
  if (
    normId.includes('playwright') ||
    normName.includes('playwright')
  ) {
    return <PlaywrightLogo size={size} className={className} style={style} />;
  }

  // 4. SQLite
  if (
    normId.includes('sqlite') ||
    normName.includes('sqlite') ||
    normId.includes('database') ||
    normName.includes('database')
  ) {
    return <SQLiteLogo size={size} className={className} style={style} />;
  }

  // 5. Context7
  if (
    normId.includes('context7') ||
    normName.includes('context7')
  ) {
    return <Context7Logo size={size} className={className} style={style} />;
  }

  // 6. Filesystem MCP / Scoped File Access
  if (
    normId.includes('filesystem') ||
    normId.includes('file_access') ||
    normId.includes('native_fs') ||
    normName.includes('filesystem') ||
    normName.includes('file access')
  ) {
    return (
      <Folder
        size={size}
        className={`inline-block shrink-0 ${className}`}
        style={{ color: '#F59E0B', ...style }}
        aria-label={title || 'Filesystem'}
      />
    );
  }

  // 7. Native Windows App Launcher
  if (
    normId.includes('launch_app') ||
    normId.includes('app_launcher') ||
    normName.includes('application')
  ) {
    return (
      <WindowsLogo
        size={size}
        className={className}
        style={{ color: '#00ADEF', ...style }}
      />
    );
  }

  // 8. Native Browser / Open URL
  if (
    normId.includes('open_url') ||
    normId.includes('browser') ||
    normName.includes('browser')
  ) {
    return (
      <Globe
        size={size}
        className={`inline-block shrink-0 ${className}`}
        style={{ color: '#38BDF8', ...style }}
        aria-label={title || 'Browser'}
      />
    );
  }

  // 9. Native Windows System Info
  if (
    normId.includes('system') ||
    normId.includes('native_system') ||
    normName.includes('system diagnostic')
  ) {
    return <WindowsLogo size={size} className={className} style={{ color: '#00ADEF', ...style }} />;
  }

  // 10. Custom MCP Server
  if (
    normId.includes('mcp_custom') ||
    normId.startsWith('mcp_') ||
    type === 'mcp'
  ) {
    return (
      <Server
        size={size}
        className={`inline-block shrink-0 ${className}`}
        style={{ color: '#7C7CF8', ...style }}
        aria-label={title || 'MCP Server'}
      />
    );
  }

  // 11. Generic API Integration
  if (
    normId.includes('api') ||
    normName.includes('api') ||
    type === 'api'
  ) {
    return (
      <Plug
        size={size}
        className={`inline-block shrink-0 ${className}`}
        style={{ color: '#38BDF8', ...style }}
        aria-label={title || 'API Integration'}
      />
    );
  }

  // Fallback
  return (
    <Layers
      size={size}
      className={`inline-block shrink-0 ${className}`}
      style={{ color: '#A5A6B0', ...style }}
      aria-label={title || 'Integration'}
    />
  );
};
