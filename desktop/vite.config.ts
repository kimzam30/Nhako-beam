import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  plugins: [svelte()],
  // The voxel mascot (design/mascot/build/mascot.glb), bundled into the 3D stage's own chunk.
  assetsInclude: ['**/*.glb'],
  // TAURI_ENV_PLATFORM is set while `tauri build` builds the UI (lib/sounds.svelte.ts reads it).
  envPrefix: ['VITE_', 'TAURI_ENV_'],
  server: { port: 5173, strictPort: true, host: true, fs: { allow: ['..'] } },
  preview: { port: 4173, strictPort: true, host: true },
  build: { target: 'es2022' },
  test: { include: ['src/**/*.test.ts'], environment: 'node' },
});
