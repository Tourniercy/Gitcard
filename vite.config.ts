import build from '@hono/vite-build/node';
import devServer from '@hono/vite-dev-server';
import { defineConfig } from 'vite';

export default defineConfig(({ mode }) => {
  if (mode === 'client') {
    throw new Error('This is a server-only project');
  }

  return {
    plugins: [
      devServer({
        entry: 'src/index.ts',
      }),
      build({
        entry: 'src/index.ts',
      }),
    ],
  };
});
