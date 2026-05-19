/**
 * # `@coding-adventures/matrix-rust-napi` — TypeScript wrapper
 *
 * **MX07 Phase 4.**  Re-exports the matrix-rust-napi Node.js N-API
 * addon with proper TypeScript declarations so consumers can use the
 * `Graph` and `Runtime` classes (and the `graphRoundTripJson` /
 * `runGraphOnCpu` legacy string-only entry points) without having to
 * hand-roll their own `.d.ts`.
 *
 * The addon itself ships as a Rust crate at
 * `code/packages/rust/matrix-rust-napi/` and produces a
 * `matrix_rust_napi.node` artifact via `npm run build` in that
 * directory.  For v0 of this wrapper, we resolve that artifact at
 * its in-repo location via `require()` (using `createRequire` since
 * this package is ESM); future versions will switch to a per-platform
 * `optionalDependencies` pattern once the publish workflow lands.
 *
 * ## What consumers see
 *
 * ```typescript
 * import { Graph, Runtime } from "@coding-adventures/matrix-rust-napi";
 *
 * const graph = new Graph(jsonString);
 * //  or:  Graph.fromJson(jsonString)
 * console.log(graph.describe());
 *
 * const rt = new Runtime();
 * //  or:  Runtime.create()
 * const outputs: Buffer[] = rt.run(graph, [inputBuf]);
 * ```
 *
 * ## How resolution works
 *
 * We use Node's `createRequire(import.meta.url)` so this ESM module
 * can `require()` the CJS `.node` file (Node still routes `.node`
 * loads through CommonJS regardless of the importing module
 * system).  Path:
 *
 * ```
 *  ../../../../packages/rust/matrix-rust-napi/matrix_rust_napi.node
 * ```
 *
 * resolved from `src/index.ts` ⇒
 * `code/packages/rust/matrix-rust-napi/matrix_rust_napi.node`.
 *
 * If the file isn't there, we throw a clear error telling the
 * caller to run `npm run build` in the Rust crate's directory.
 */

import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import { existsSync } from "node:fs";

// ─────────────────────────────────────────────────────────────────────────────
// Public types
//
// Mirror the four exports of the Rust addon.  Keeping these in TS
// rather than .d.ts means consumers' editors give immediate hover-doc
// without having to publish a separate types package.
// ─────────────────────────────────────────────────────────────────────────────

/**
 * The matrix-IR JSON wire format version this addon's classes accept.
 * Equal to `matrix_ir::WIRE_FORMAT_VERSION` on the Rust side.  Bumped
 * only when the JSON schema makes a breaking change.
 */
export const MATRIX_IR_VERSION = 1 as const;

/**
 * A parsed matrix-IR graph held behind a Rust-owned `Box<Graph>`.
 * Constructed from the canonical JSON wire format (see
 * [`matrix-ir-json`](../../rust/matrix-ir-json/README.md) for the
 * schema).  Reused across many `Runtime.run` calls without re-
 * parsing the JSON.
 */
export interface Graph {
  /**
   * Re-serialise back to the matrix-ir-json wire format.  The output
   * is byte-equal to the input *modulo* whitespace and key order
   * (decoders normalise both).
   */
  toJson(): string;

  /**
   * Short human-readable summary:
   * `"Graph(tensors=4, ops=3, inputs=1, outputs=1, constants=2)"`.
   * Useful for log messages and assertions.
   */
  describe(): string;
}

/**
 * Constructor for [`Graph`].  Parses `jsonString` immediately;
 * throws on schema validation failure (unknown op kind, malformed
 * hex in a constant, version mismatch, …).
 */
export interface GraphConstructor {
  new (jsonString: string): Graph;
  /**
   * Static-method sugar for `new Graph(jsonString)`.  Semantically
   * identical; matches the MX07 spec's `Graph.fromJson` shape.
   */
  fromJson(jsonString: string): Graph;
}

/**
 * Owns the planner + CPU executor.  In v0 it is stateless (each
 * `run` call constructs a fresh `matrix_runtime::Runtime` +
 * `matrix_cpu::CpuExecutor`); the class wrapping keeps the JS-side
 * API stable so consumers won't have to migrate when v1 adds
 * persistent state (option flags, executor pool, …).
 */
export interface Runtime {
  /**
   * Plan and execute `graph` on the CPU executor.  Each
   * `inputs[i]` provides the little-endian byte payload for the
   * corresponding `graph.inputs()` tensor in declaration order.
   * Returns one `Buffer` per `graph.outputs()` tensor.
   *
   * Throws on:
   *   - input count mismatch
   *   - input byte-length mismatch (wrong dtype or shape on the JS side)
   *   - total graph buffer size exceeding 4 GiB
   *     (`MAX_TOTAL_BUFFER_BYTES`; DoS-defence cap)
   *   - any internal planner / executor failure
   */
  run(graph: Graph, inputs: Buffer[]): Buffer[];
}

/**
 * Constructor for [`Runtime`].  Takes no arguments; the constructor
 * exists primarily to give consumers a value to call `.run` on.
 */
export interface RuntimeConstructor {
  new (): Runtime;
  /**
   * Static-method sugar for `new Runtime()`.  Semantically
   * identical; matches the MX07 spec's `Runtime.create` shape.
   */
  create(): Runtime;
}

/**
 * The shape of the loaded native addon.
 */
interface MatrixRustNapi {
  /**
   * Round-trip a graph through `matrix-ir-json::decode` followed by
   * `matrix-ir-json::encode`.  Useful as a JSON-schema validator
   * (throws on malformed input).
   */
  graphRoundTripJson(jsonString: string): string;

  /**
   * One-shot JSON-envelope execution path.  Envelope shape:
   * `{ graph: <matrix-ir-json schema>, inputs: ["<lowercase-hex>", ...] }`
   * → `{ outputs: ["<lowercase-hex>", ...] }`.  Kept for callers
   * without `Buffer` (CLI pipes, golden-file fixtures); the
   * class-based `Graph` + `Runtime` API is preferred for any
   * Node-side consumer.
   */
  runGraphOnCpu(envelopeJson: string): string;

  /** The `Graph` class — see [`Graph`] / [`GraphConstructor`]. */
  Graph: GraphConstructor;

  /** The `Runtime` class — see [`Runtime`] / [`RuntimeConstructor`]. */
  Runtime: RuntimeConstructor;
}

// ─────────────────────────────────────────────────────────────────────────────
// Addon resolution
//
// `createRequire(import.meta.url)` is the canonical way to call the
// CommonJS `require()` from an ESM module.  Node always routes
// `.node` files through CJS regardless of the importer; ESM `import`
// of a native addon would throw.
//
// We resolve the addon relative to this file's location.  At install
// time the wrapper hasn't been compiled yet (src/index.ts is shipped
// directly because package.json's `main` points there), so `__dirname`
// equivalent is `src/`.  Up four levels lands at
// `code/packages/typescript/` → up one more is `code/packages/` →
// then back down to `rust/matrix-rust-napi/`.
// ─────────────────────────────────────────────────────────────────────────────

const here = dirname(fileURLToPath(import.meta.url));

/**
 * Candidate addon paths, in resolution order.
 *
 * 1. **Colocated**: `<package-root>/matrix_rust_napi.node`.  The
 *    `BUILD` script in this package builds the Rust addon and copies
 *    the platform-specific shared library here, so this is the
 *    expected location after `./BUILD` runs.  Also where a future
 *    per-platform npm publishing flow would land the prebuilt binary.
 *
 * 2. **Rust-crate fallback**:
 *    `code/packages/rust/matrix-rust-napi/matrix_rust_napi.node`.  The
 *    Rust crate's own `npm run build` (MX07 Phase 3) drops the
 *    `.node` file there for its own load-smoke step.  Resolving to
 *    that location lets a developer who only built via the Rust
 *    side still drive this wrapper's smoke tests without having to
 *    rerun the TS package's `BUILD`.
 */
const ADDON_CANDIDATES = [
  resolve(here, "..", "matrix_rust_napi.node"),
  resolve(
    here,
    "..",
    "..",
    "..",
    "rust",
    "matrix-rust-napi",
    "matrix_rust_napi.node",
  ),
];

/**
 * Lazy + memoised addon loader.  Defers the `require()` call until
 * first access so that simply importing this module (e.g. for the
 * exported types) doesn't crash on a fresh checkout where the
 * `.node` artifact hasn't been built yet.  Throws a precise,
 * actionable error listing every candidate path it tried.
 */
let cached: MatrixRustNapi | null = null;
function loadAddon(): MatrixRustNapi {
  if (cached !== null) return cached;

  const found = ADDON_CANDIDATES.find((p) => existsSync(p));
  if (!found) {
    throw new Error(
      `matrix_rust_napi addon not found.  Looked at:\n` +
        ADDON_CANDIDATES.map((p) => `  - ${p}`).join("\n") +
        `\n\nRun \`./BUILD\` in code/packages/typescript/matrix-rust-napi/, ` +
        `or \`npm run build\` in code/packages/rust/matrix-rust-napi/.`,
    );
  }

  const req = createRequire(import.meta.url);
  cached = req(found) as MatrixRustNapi;
  return cached;
}

// ─────────────────────────────────────────────────────────────────────────────
// Public re-exports
//
// We expose the addon's four entry points behind getter functions
// (for the classes) and direct delegates (for the legacy string
// functions).  This indirection keeps the addon load lazy and lets
// the TypeScript compiler enforce the typed interface above.
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Validate a JSON graph payload via round-trip — decode then re-encode.
 * Returns the normalised JSON.  Throws on malformed input.
 */
export function graphRoundTripJson(jsonString: string): string {
  return loadAddon().graphRoundTripJson(jsonString);
}

/**
 * One-shot JSON-envelope execution path.  See [`MatrixRustNapi.runGraphOnCpu`].
 * Kept for CLI / no-Buffer consumers; the `Graph` + `Runtime` class
 * API is preferred otherwise.
 */
export function runGraphOnCpu(envelopeJson: string): string {
  return loadAddon().runGraphOnCpu(envelopeJson);
}

/**
 * The `Graph` class.  Accessed via a getter so the addon load stays
 * lazy until consumers actually instantiate one.
 */
export const Graph: GraphConstructor = new Proxy(
  function Graph() {} as unknown as GraphConstructor,
  {
    construct(_target, args) {
      const Ctor = loadAddon().Graph;
      return new Ctor(args[0] as string);
    },
    get(_target, prop, _recv) {
      if (prop === "fromJson") {
        return (json: string) => loadAddon().Graph.fromJson(json);
      }
      return undefined;
    },
  },
);

/**
 * The `Runtime` class.  Accessed via a getter so the addon load
 * stays lazy until consumers actually instantiate one.
 */
export const Runtime: RuntimeConstructor = new Proxy(
  function Runtime() {} as unknown as RuntimeConstructor,
  {
    construct(_target, _args) {
      const Ctor = loadAddon().Runtime;
      return new Ctor();
    },
    get(_target, prop, _recv) {
      if (prop === "create") {
        return () => loadAddon().Runtime.create();
      }
      return undefined;
    },
  },
);
