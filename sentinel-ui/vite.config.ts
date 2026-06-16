import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import * as fs from 'fs';
import * as path from 'path';

export default defineConfig({
  plugins: [
    react(),
    {
      name: 'write-port-to-file',
      configureServer(server) {
        server.httpServer?.once('listening', () => {
          const address = server.httpServer?.address();
          if (address && typeof address === 'object') {
            const port = address.port;
            fs.writeFileSync(path.join(__dirname, '.port'), port.toString(), 'utf-8');
            console.log(`[Vite Plugin] Wrote port ${port} to .port file`);
          }
        });
      }
    }
  ],
  base: './', // important for electron
  server: {
    port: 51793,
    strictPort: false, // Allow iterating to next port if in use
  },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
  }
});
