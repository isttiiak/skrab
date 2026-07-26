import { defineConfig } from 'astro/config';

export default defineConfig({
  site: 'https://skrab.pages.dev',
  // Fully static: the download page resolves release assets at build time and the
  // release workflow triggers a rebuild, so nothing here needs a server.
  output: 'static',
  build: {
    inlineStylesheets: 'auto',
  },
  devToolbar: {
    enabled: false,
  },
});
