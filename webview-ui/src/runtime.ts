/**
 * Runtime detection, provider-agnostic
 *
 * Single source of truth for determining whether the webview is running
 * inside an IDE extension (VS Code, Cursor, Windsurf, etc.), standalone
 * in a browser, or inside a Tauri desktop app.
 */

declare function acquireVsCodeApi(): unknown;

export type Runtime = 'vscode' | 'browser' | 'tauri';
// Future: 'cursor' | 'windsurf' | 'electron' | etc.

function detectRuntime(): Runtime {
  // Tauri injects __TAURI_INTERNALS__ into the window object
  if (typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window) {
    return 'tauri';
  }
  if (typeof acquireVsCodeApi !== 'undefined') {
    return 'vscode';
  }
  return 'browser';
}

export const runtime: Runtime = detectRuntime();

export const isBrowserRuntime = runtime === 'browser';
export const isTauriRuntime = runtime === 'tauri';
export const isVscodeRuntime = runtime === 'vscode';
