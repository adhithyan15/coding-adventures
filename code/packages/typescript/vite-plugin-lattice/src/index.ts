/**
 * @coding-adventures/vite-plugin-lattice
 *
 * Vite plugin that transpiles Lattice CSS superset files (.lattice) to
 * plain CSS at build time. Lattice extends CSS with variables, mixins,
 * functions, and control flow.
 *
 * Usage in vite.config.ts:
 *
 *   import { latticePlugin } from "@coding-adventures/vite-plugin-lattice";
 *
 *   export default defineConfig({
 *     plugins: [latticePlugin()],
 *   });
 *
 * Then import .lattice files in your source:
 *
 *   import "./styles/app.lattice";
 *
 * The plugin handles:
 *   - Transform: .lattice → CSS at build time
 *   - HMR: Instant style updates during development
 *   - Style injection: CSS is injected via <style> tags in dev mode
 *     and extracted to separate .css files in production builds
 */

export { latticePlugin } from "./plugin.js";
export type { LatticePluginOptions } from "./plugin.js";
