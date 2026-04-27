/**
 * plugin.ts — Vite plugin for .lattice file transformation.
 *
 * This plugin teaches Vite how to handle `.lattice` file imports.
 *
 * === Node .ts resolution challenge ===
 *
 * This monorepo's packages use TypeScript source files as their entry points
 * (e.g., "main": "src/index.ts"). This works for Vite's client-side module
 * resolution (which uses esbuild), but NOT for Node.js server-side code.
 *
 * For the Vite dev server, plugins run in Node.js. When the plugin needs to
 * call the lattice-transpiler, it must resolve the .ts file through Vite's
 * SSR module loader (ssrLoadModule), which applies esbuild transforms.
 *
 * We resolve the transpiler path relative to this plugin's package directory,
 * using the known sibling package layout of the monorepo.
 */

import { readFileSync } from "fs";
import { resolve, dirname } from "path";
import { fileURLToPath } from "url";
import { type Plugin, type ViteDevServer, createServer } from "vite";

const __dirname_plugin = dirname(fileURLToPath(import.meta.url));
const TRANSPILER_PATH = resolve(__dirname_plugin, "../../lattice-transpiler/src/browser.ts");

/**
 * Options for the Lattice Vite plugin.
 */
export interface LatticePluginOptions {
  /** Emit minified CSS. Default: false */
  minified?: boolean;
  /** Indentation string. Default: "  " */
  indent?: string;
}

/**
 * latticePlugin — factory function that creates the Vite plugin.
 */
export function latticePlugin(options: LatticePluginOptions = {}): Plugin {
  const { minified = false, indent = "  " } = options;

  let transpile: ((source: string, opts?: { minified?: boolean; indent?: string }) => string) | null = null;
  let server: ViteDevServer | null = null;
  let isBuildModeServer = false;

  return {
    name: "vite-plugin-lattice",

    configureServer(srv) {
      server = srv;
    },

    async transform(code: string, id: string) {
      if (!id.endsWith(".lattice")) return null;

      try {
        if (!transpile) {
          if (!server) {
            // Build mode fallback: create a headless Vite server just for ssrLoadModule
            server = await createServer({
              server: { middlewareMode: true },
              appType: "custom",
              optimizeDeps: { noDiscovery: true },
            });
            isBuildModeServer = true;
          }
          const mod = await server.ssrLoadModule(TRANSPILER_PATH);
          transpile = mod.transpileLatticeInBrowser;
        }

        const css = transpile!(code, { minified, indent });

        const jsCode = `
const css = ${JSON.stringify(css)};
const style = document.createElement("style");
style.setAttribute("data-lattice", "");
style.textContent = css;
document.head.appendChild(style);
export default css;
`;
        return { code: jsCode, map: null };
      } catch (err) {
        const message = err instanceof Error ? err.message : "Lattice compilation error";
        this.error(`Lattice compilation error in ${id}: ${message}`);
      }
    },

    async handleHotUpdate(ctx) {
      if (!ctx.file.endsWith(".lattice")) return;

      const source = readFileSync(ctx.file, "utf-8");
      try {
        if (!transpile) {
          if (!server) {
            server = await createServer({
              server: { middlewareMode: true },
              appType: "custom",
              optimizeDeps: { noDiscovery: true },
            });
            isBuildModeServer = true;
          }
          const mod = await server.ssrLoadModule(TRANSPILER_PATH);
          transpile = mod.transpileLatticeInBrowser;
        }
        transpile!(source, { minified, indent });
        return ctx.modules;
      } catch {
        console.error(`[lattice] Compilation error in ${ctx.file}`);
        return [];
      }
    },
    async buildEnd() {
      if (isBuildModeServer && server) {
        await server.close();
      }
    }
  };
}
