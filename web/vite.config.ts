import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

function manualChunks(id: string) {
  if (!id.includes("node_modules")) {
    return;
  }
  if (id.includes("/antd/") || id.includes("/@ant-design/icons/")) {
    return "antd";
  }
  if (id.includes("/react/") || id.includes("/react-dom/")) {
    return "react";
  }
  return "vendor";
}

// biome-ignore lint/style/noDefaultExport: Vite config files are conventionally loaded from a default export.
export default defineConfig({
  build: {
    chunkSizeWarningLimit: 1200,
    rollupOptions: {
      output: {
        manualChunks,
      },
    },
  },
  plugins: [react()],
  server: {
    port: 5173,
    proxy: {
      "/api": "http://127.0.0.1:8787",
    },
  },
});
