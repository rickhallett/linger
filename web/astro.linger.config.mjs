import { defineConfig } from 'astro/config';

// Keep the inherited Zoetrope browser app and assets available for provenance,
// but build only the canonical Linger website.
export default defineConfig({
  site: 'https://lingerer.xyz',
  srcDir: './linger',
  publicDir: './linger/public',
  outDir: './dist',
  devToolbar: { enabled: false },
});
