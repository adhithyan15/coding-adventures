// index.js -- JavaScript entry point for the native addon
// ========================================================
//
// This file loads the compiled Rust native addon (.node file) and re-exports
// the DirectedGraph class. The napi-rs build step compiles Rust code into a
// platform-specific .node file (e.g., directed-graph-native.win32-x64-msvc.node).
//
// The @napi-rs/cli `napi build` command generates the .node file and a
// loader that picks the right binary for the current platform.
//
// If you're reading this and wondering "where's the actual code?", it's in
// src/lib.rs -- this file is just the loading mechanism.
//
// We use createRequire because the package is ESM ("type": "module") but
// .node files can only be loaded via require(). This is the standard pattern
// for loading native addons from ESM packages.

import { createRequire } from "module";

const require = createRequire(import.meta.url);
const { DirectedGraph } = require("./directed-graph-native.node");

export { DirectedGraph };
