/**
 * Tauri runtime adapter — bridges Tauri IPC events to the same MessageEvent
 * pattern used by VS Code's postMessage, so useExtensionMessages.ts works
 * without changes.
 *
 * Only imported when runtime === 'tauri'; tree-shaken from other runtimes.
 */

// Tauri event types — Tauri events emit { event, payload } where payload is our data
interface TauriEvent<T> {
  event: string;
  payload: T;
}

type UnlistenFn = () => void;
type ListenFn = <T>(event: string, handler: (event: TauriEvent<T>) => void) => Promise<UnlistenFn>;
type InvokeFn = (cmd: string, args?: Record<string, unknown>) => Promise<unknown>;

// We dynamically import @tauri-apps/api to avoid bundling it in non-Tauri builds
let listen: ListenFn;
let invoke: InvokeFn;

function dispatch(data: unknown): void {
  window.dispatchEvent(new MessageEvent('message', { data }));
}

const TAURI_EVENTS = [
  'agent-created',
  'agent-closed',
  'agent-tool-start',
  'agent-tool-done',
  'agent-tools-clear',
  'agent-status',
  'agent-tool-permission',
  'agent-tool-permission-clear',
  'subagent-tool-start',
  'subagent-tool-done',
  'subagent-clear',
  'subagent-tool-permission',
  'existing-agents',
  'layout-loaded',
  'settings-loaded',
  'character-sprites-loaded',
  'floor-tiles-loaded',
  'wall-tiles-loaded',
  'furniture-assets-loaded',
  'workspace-folders',
] as const;

/**
 * Initialize the Tauri backend adapter.
 * Registers listeners for all backend events and dispatches them as
 * synthetic MessageEvents on window.
 */
export async function initTauriBackend(): Promise<void> {
  console.log('[TauriAdapter] Initializing...');

  // Dynamic import to avoid bundling in non-Tauri builds
  const eventModule = await import('@tauri-apps/api/event');
  const coreModule = await import('@tauri-apps/api/core');

  listen = eventModule.listen as unknown as ListenFn;
  invoke = coreModule.invoke;

  // Register listeners for all backend events
  for (const eventName of TAURI_EVENTS) {
    await listen(eventName, (event: TauriEvent<unknown>) => {
      // The payload IS the message data — dispatch directly
      dispatch(event.payload);
    });
  }

  console.log(`[TauriAdapter] Registered ${TAURI_EVENTS.length} event listeners`);
}

/**
 * Send a message to the Tauri backend.
 * Maps the VS Code postMessage protocol to Tauri invoke() commands.
 */
export function tauriSend(msg: Record<string, unknown>): void {
  const msgType = msg.type as string;

  switch (msgType) {
    case 'webviewReady':
      invoke('webview_ready').catch(console.error);
      break;
    case 'saveLayout':
      invoke('save_layout', { layout: msg.layout }).catch(console.error);
      break;
    case 'saveAgentSeats':
      invoke('save_agent_seats', { seats: msg.seats }).catch(console.error);
      break;
    case 'setSoundEnabled':
      invoke('set_sound_enabled', { enabled: msg.enabled }).catch(console.error);
      break;
    case 'exportLayout':
      // In Tauri, dialog is handled by the frontend before calling export
      if (msg.path) {
        invoke('export_layout', { path: msg.path }).catch(console.error);
      }
      break;
    case 'importLayout':
      if (msg.path) {
        invoke('import_layout', { path: msg.path }).catch(console.error);
      }
      break;
    case 'openSessionsFolder':
      invoke('open_sessions_folder').catch(console.error);
      break;
    case 'closeAgent':
      invoke('close_agent', { id: msg.id }).catch(console.error);
      break;
    case 'focusAgent':
      // No-op in Tauri (passive monitoring, no terminal to focus)
      break;
    case 'openClaude':
      // No-op in Tauri (passive monitoring)
      break;
    default:
      console.log('[TauriAdapter] Unhandled message type:', msgType, msg);
  }
}
