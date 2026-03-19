import './index.css';

import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';

import App from './App.tsx';
import { isBrowserRuntime, isTauriRuntime } from './runtime';

async function main() {
  if (isTauriRuntime) {
    // Tauri runtime: initialize backend adapter, then load assets via browser mock
    // (Tauri serves static assets the same way as a browser build)
    const { initTauriBackend, tauriSend } = await import('./tauriAdapter.js');
    const { setTauriSendFn } = await import('./vscodeApi.js');
    setTauriSendFn(tauriSend);
    await initTauriBackend();
    // Load assets using the browser mock (Tauri serves them as static files)
    const { initBrowserMock } = await import('./browserMock.js');
    await initBrowserMock();
  } else if (isBrowserRuntime) {
    const { initBrowserMock } = await import('./browserMock.js');
    await initBrowserMock();
  }
  createRoot(document.getElementById('root')!).render(
    <StrictMode>
      <App />
    </StrictMode>,
  );
}

main().catch(console.error);
