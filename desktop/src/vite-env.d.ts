/// <reference types="svelte" />
/// <reference types="vite/client" />

interface ImportMetaEnv {
  /** Set while `tauri build` builds the UI for the desktop app. */
  readonly TAURI_ENV_PLATFORM?: string;
}
