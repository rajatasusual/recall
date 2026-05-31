import { getCurrentWindow } from "@tauri-apps/api/window";

const appWindow = getCurrentWindow();

export function TitleBar() {
  return (
    <header class="titlebar" data-tauri-drag-region>
      <span class="app-name">Recall</span>
      <div class="win-controls">
        <button class="wc wc-min" onClick={() => appWindow.minimize()} title="Minimise">
          <svg viewBox="0 0 10 1"><rect width="10" height="1" stroke="currentColor" stroke-width="1.4" /></svg>
        </button>
        <button class="wc wc-max" onClick={() => appWindow.toggleMaximize()} title="Maximise">
          <svg viewBox="0 0 10 10"><rect x="1" y="1" width="8" height="8" fill="none" stroke="currentColor" stroke-width="1.2" /></svg>
        </button>
        <button class="wc wc-close" onClick={() => appWindow.close()} title="Close">
          <svg viewBox="0 0 10 10">
            <line x1="1" y1="1" x2="9" y2="9" stroke="currentColor" stroke-width="1.4" />
            <line x1="9" y1="1" x2="1" y2="9" stroke="currentColor" stroke-width="1.4" />
          </svg>
        </button>
      </div>
    </header>
  );
}