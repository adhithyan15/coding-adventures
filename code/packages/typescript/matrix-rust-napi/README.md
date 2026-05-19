# @coding-adventures/matrix-rust-napi (TypeScript)

TypeScript wrapper for the [matrix-rust-napi](../../rust/matrix-rust-napi/)
Node.js N-API addon.  Provides typed re-exports of the addon's
four entry points — `Graph` class, `Runtime` class, plus the legacy
`graphRoundTripJson` / `runGraphOnCpu` string-only functions — and
ships the end-to-end vitest smoke suite that drives the addon
through real JS (Buffer-based MatMul, Add, error paths).

This is **MX07 Phase 4** in the ARCH02 rollout.

## Install + build

```sh
cd code/packages/typescript/matrix-rust-napi
npm install
```

The wrapper has no runtime dependencies — only typescript + vitest +
@types/node as devDeps.

To run the tests you need the Rust `.node` artifact built first:

```sh
cd ../../rust/matrix-rust-napi && npm run build
```

The wrapper resolves the addon at
`code/packages/rust/matrix-rust-napi/matrix_rust_napi.node`
(relative to `src/index.ts`).  If the file is missing the wrapper
throws a precise error explaining how to build it.

The `BUILD` script in this directory handles both steps for CI:
it `cd`s into the Rust crate, runs `npm run build` there, then
runs `npx vitest run --coverage` here.

## Usage

```typescript
import { Graph, Runtime } from "@coding-adventures/matrix-rust-napi";

// Build a small matrix-ir graph in JSON (see matrix-ir-json for the
// schema, or use matrix-ir's GraphBuilder + matrix-ir-json::encode
// on the Rust side and ship the JSON string over).
const json = JSON.stringify({
  matrix_ir_version: 1,
  tensors: [
    { id: 0, dtype: "f32", shape: [3] },
    { id: 1, dtype: "f32", shape: [3] },
    { id: 2, dtype: "f32", shape: [3] },
  ],
  inputs: [0, 1],
  outputs: [2],
  ops: [{ kind: "Add", lhs: 0, rhs: 1, output: 2 }],
  constants: [],
});

const graph = new Graph(json);
//  or:  Graph.fromJson(json)
console.log(graph.describe());
//   "Graph(tensors=3, ops=1, inputs=2, outputs=1, constants=0)"

const rt = new Runtime();
//  or:  Runtime.create()

const a = Buffer.alloc(12);  // 3 × f32 = 12 bytes
const b = Buffer.alloc(12);
for (let i = 0; i < 3; i++) {
  a.writeFloatLE(i + 1, i * 4);          // [1, 2, 3]
  b.writeFloatLE((i + 1) * 10, i * 4);    // [10, 20, 30]
}

const [output] = rt.run(graph, [a, b]);
// output is a Buffer; read as f32s:
for (let i = 0; i < 3; i++) console.log(output.readFloatLE(i * 4));
// 11, 22, 33
```

## Layered API

The addon exposes two parallel APIs, both ultimately routing
through the same `run_graph_on_cpu` Rust helper:

| API | Best for | Cost |
|-----|----------|------|
| `Graph` + `Runtime` classes, `Buffer[]` I/O | Node consumers with real `Buffer`; hot loops that reuse a parsed graph | One JSON parse on `new Graph`; zero-overhead bytes through `Buffer` |
| `runGraphOnCpu(envelopeJson)` | CLI tools / language hosts without `Buffer`; golden-file fixtures | JSON parse + hex encode/decode per call |

The string-only `graphRoundTripJson(json)` rounds the graph through
`matrix-ir-json::decode` + `encode` — useful as a schema validator.

## Testing

```sh
npm run test               # 15 vitest tests
npm run test:coverage      # with c8 line-coverage report
```

The smoke suite covers:

* **Graph class**: parse + describe, `toJson()` round-trip, `fromJson` static
  equivalence, malformed JSON rejection, unsupported version rejection.
* **Runtime class**: `Runtime.create()` factory, element-wise Add on f32×3,
  2×2 MatMul (numerical correctness), wrong arity / wrong byte length /
  non-Buffer input error paths.
* **`graphRoundTripJson`**: idempotence + malformed-input rejection.
* **`runGraphOnCpu` JSON envelope**: hex-encoded Add round-trip +
  schema-missing-graph rejection.

## License

MIT
