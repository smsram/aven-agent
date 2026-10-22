/**
 * Native desktop behavior optimizations for Aven Tauri desktop application:
 * - Disables unwanted browser-style context menus
 * - Prevents accidental browser navigation (Alt+Arrow, Backspace outside inputs)
 * - Prevents accidental page reload (F5, Ctrl+R)
 * - Prevents webview developer shortcuts (Ctrl+U, Ctrl+P, F12 in non-dev)
 * - Preserves essential text editing shortcuts (Ctrl+C, Ctrl+V, Ctrl+X, Ctrl+A, Ctrl+Z, Ctrl+Y)
 */
export function initNativeDesktopBehavior() {
  if (typeof window === 'undefined') return;

  // 1. Disable browser page context menu on inputs and all other places
  window.addEventListener(
    'contextmenu',
    (e) => {
      e.preventDefault();
    },
    { capture: true }
  );

  // 2. Suppress unwanted browser navigation and reload shortcuts
  window.addEventListener(
    'keydown',
    (e: KeyboardEvent) => {
      const key = e.key.toLowerCase();
      const isCtrlOrMeta = e.ctrlKey || e.metaKey;

      // Allow essential text editing shortcuts
      if (isCtrlOrMeta && !e.shiftKey && ['c', 'v', 'x', 'a', 'z', 'y'].includes(key)) {
        return; // Always preserve copy, paste, cut, select-all, undo, redo
      }
      if (isCtrlOrMeta && e.shiftKey && key === 'z') {
        return; // Preserve redo (Ctrl+Shift+Z)
      }

      // Prevent F5 or Ctrl+R (Accidental Reload)
      if (e.key === 'F5' || (isCtrlOrMeta && key === 'r')) {
        e.preventDefault();
        return;
      }

      // Prevent Alt+Left / Alt+Right (Browser Back/Forward navigation)
      if (e.altKey && (e.key === 'ArrowLeft' || e.key === 'ArrowRight')) {
        e.preventDefault();
        return;
      }

      // Prevent Backspace navigating back when not inside input/textarea
      if (e.key === 'Backspace') {
        const target = e.target as HTMLElement | null;
        const isInput =
          target &&
          (target.tagName === 'INPUT' ||
            target.tagName === 'TEXTAREA' ||
            target.isContentEditable);
        if (!isInput) {
          e.preventDefault();
          return;
        }
      }

      // Prevent Ctrl+U (View Source), Ctrl+P (Browser Print)
      if (isCtrlOrMeta && (key === 'u' || key === 'p')) {
        e.preventDefault();
        return;
      }

      // Prevent F12 or Ctrl+Shift+I / Ctrl+Shift+J (DevTools shortcut suppression in desktop app)
      if (
        e.key === 'F12' ||
        (isCtrlOrMeta && e.shiftKey && (key === 'i' || key === 'j'))
      ) {
        // Prevent accidental devtools trigger
        e.preventDefault();
        return;
      }
    },
    true
  );
}
