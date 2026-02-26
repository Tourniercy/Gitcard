import build from '@hono/vite-build/node';
import devServer from '@hono/vite-dev-server';
import { defineConfig, loadEnv } from 'vite';

export default defineConfig(({ mode }) => {
  if (mode === 'client') {
    throw new Error('This is a server-only project');
  }

  // Load .env into process.env so config.ts can read PAT_1, PORT, etc.
  const env = loadEnv(mode, process.cwd(), '');
  Object.assign(process.env, env);

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
