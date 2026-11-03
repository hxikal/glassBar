/// <reference types="vitest/config" />
import { defineConfig } from 'vite';
import { configDefaults } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  build: { target: 'es2022' },
  // Other sessions' git worktrees live under .claude/: their copies of the tests are not ours.
  test: { exclude: [...configDefaults.exclude, '.claude/**'] },
});
