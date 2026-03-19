import { isBrowserRuntime, isTauriRuntime } from './runtime';

declare function acquireVsCodeApi(): { postMessage(msg: unknown): void };

// Lazy-loaded Tauri send function (populated after initTauriBackend)
let tauriSendFn: ((msg: Record<string, unknown>) => void) | null = null;

/** Called by main.tsx after initTauriBackend() to wire up tauriSend */
export function setTauriSendFn(fn: (msg: Record<string, unknown>) => void): void {
  tauriSendFn = fn;
}

function createPostMessage(): (msg: unknown) => void {
  if (isTauriRuntime) {
    return (msg: unknown) => {
      if (tauriSendFn) {
        tauriSendFn(msg as Record<string, unknown>);
      } else {
        console.warn('[vscodeApi] tauriSend not yet initialized, queuing message:', msg);
      }
    };
  }
  if (isBrowserRuntime) {
    return (msg: unknown) => console.log('[vscode.postMessage]', msg);
  }
  // VS Code runtime
  const api = acquireVsCodeApi();
  return (msg: unknown) => api.postMessage(msg);
}

export const vscode: { postMessage(msg: unknown): void } = {
  postMessage: createPostMessage(),
};
