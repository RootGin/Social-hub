import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { readFileSync } from "node:fs";
import { compile } from "svelte/compiler";

// ponytail: svelte plugin returns undefined from load() when a style query
// hits a cold cache. Vite then reads the .svelte file as if it were CSS and
// PostCSS chokes on the <script> block. We compile the file ourselves and
// return the extracted CSS directly.
function svelteStyleFallback() {
  return {
    name: "svelte-style-fallback",
    enforce: "pre",
    load(id) {
      const queryIdx = id.indexOf("?svelte&type=style");
      if (queryIdx === -1) return;
      const filename = id.slice(0, queryIdx);
      const source = readFileSync(filename, "utf-8");
      const { css } = compile(source, { filename, generate: "dom", css: "external" });
      return css || "/* empty */";
    },
  };
}

export default defineConfig({
  plugins: [svelteStyleFallback(), svelte()],

  clearScreen: false,

  server: {
    port: 5173,
    strictPort: true,
    // ponytail: Vite binds IPv6-only by default. The Tauri webview resolves
    // localhost to 127.0.0.1 first and fails to connect. Bind IPv4 explicitly.
    host: "127.0.0.1",
  },

  envPrefix: ["VITE_", "TAURI_"],

  build: {
    target: "esnext",
    minify: !process.env.TAURI_DEBUG ? "esbuild" : false,
    sourcemap: !!process.env.TAURI_DEBUG,
  },
});
