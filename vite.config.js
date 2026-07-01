import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { compile } from "svelte/compiler";

// ponytail: svelte plugin returns undefined from load() for a cold-cache
// ?type=style query. Vite then reads the .svelte file as CSS and PostCSS
// chokes on the <script> block. We compile the file ourselves with the
// same cssHash as the svelte plugin so the CSS class names match.
function svelteStyleFallback() {
  let root = "";
  return {
    name: "svelte-style-fallback",
    enforce: "pre",
    configResolved(config) {
      root = config.root;
    },
    load(id) {
      const queryIdx = id.indexOf("?svelte&type=style");
      if (queryIdx === -1) return;
      const fullPath = id.slice(0, queryIdx);
      // normalizedFilename = strip root prefix (matching id.js normalize())
      const normalized = fullPath.startsWith(root + "/") ? fullPath.slice(root.length) : fullPath;
      // replicate @sveltejs/vite-plugin-svelte's dev-mode cssHash
      // MD5 → base64 → URL-safe (MUST match hash.js exactly)
      const b64 = createHash("md5").update(normalized).digest("base64");
      const safe = b64.replace(/[+/=]/g, (c) => ({ "+": "-", "/": "_", "=": "" })[c]);
      const hash = "s-" + safe.slice(0, 12);
      const source = readFileSync(fullPath, "utf-8");
      const { css } = compile(source, {
        filename: fullPath,
        generate: "dom",
        css: "external",
        cssHash: () => hash,
      });
      return css || "/* empty */";
    },
  };
}

export default defineConfig(({ command }) => ({
  // ponytail: svelteStyleFallback is dev-only — it patches a cold-cache
  // bug in the official plugin for `?svelte&type=style` queries. In
  // production builds the official plugin handles it correctly, AND
  // the fallback's hardcoded `s-` hash prefix mismatches the official
  // plugin's `svelte-` prefix when HMR is off, breaking all CSS scoping.
  plugins: command === "build" ? [svelte()] : [svelteStyleFallback(), svelte()],

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
}));
