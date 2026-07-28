import { defineConfig } from 'vitest/config';
import path from 'node:path';

// Workspace packages resolve via pnpm-managed node_modules symlinks in a
// full install; these aliases let this package's tests run standalone too.
export default defineConfig({
  resolve: {
    alias: {
      '@hyperchess/core': path.resolve(__dirname, '../core/src/index.ts'),
      '@hyperchess/theme': path.resolve(__dirname, '../theme/src/index.ts'),
    },
  },
});
