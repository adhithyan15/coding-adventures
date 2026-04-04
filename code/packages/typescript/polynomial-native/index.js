// index.js -- Load the native polynomial addon
// =============================================
//
// This module loads the compiled Rust .node binary using Node's
// createRequire trick. ESM modules can't use require() directly,
// so we create a CommonJS require function from import.meta.url
// and use it to load the native addon.
//
// The .node file is a dynamic library (DLL on Windows, .so on Linux,
// .dylib on macOS) that Node.js loads via its N-API addon mechanism.

import { createRequire } from "module";
import { dirname, join } from "path";
import { fileURLToPath } from "url";

// createRequire gives us a CJS-style require() that can load .node files.
// We point it at this file's directory so the path resolution is correct.
const __dirname = dirname(fileURLToPath(import.meta.url));
const require = createRequire(join(__dirname, "package.json"));

// Load the native addon. The file is named after the Cargo crate with
// hyphens replaced by underscores (Cargo convention for cdylib output).
const native = require("./polynomial_native_node.node");

// Re-export all polynomial functions so consumers can do:
//   import { normalize, add, multiply } from "@coding-adventures/polynomial-native";
export const {
  normalize,
  degree,
  zero,
  one,
  add,
  subtract,
  multiply,
  divmodPoly,
  divide,
  modulo,
  evaluate,
  gcd,
} = native;
