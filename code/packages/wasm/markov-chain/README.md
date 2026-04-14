# markov-chain-wasm

WebAssembly bindings for the [`coding-adventures-markov-chain`](../../rust/markov-chain) Rust crate (DT28).

Exposes `WasmMarkovChain` to JavaScript/TypeScript via `wasm-bindgen`.

## Stack position

```
JS / TypeScript consumer
        ↓  (WebAssembly boundary)
markov-chain-wasm          ← this package
        ↓  (Rust crate dep)
coding-adventures-markov-chain   (DT28 Rust implementation)
        ↓  (Rust crate dep)
directed-graph             (DT01 graph topology)
```

## Quick start

```js
import init, { WasmMarkovChain } from './markov_chain_wasm.js';
await init();

// Build a character-level text generator
const chain = new WasmMarkovChain(2, 1.0, []);
chain.trainString("the quick brown fox jumps over the lazy dog ".repeat(50));

console.log(chain.generateString("th", 80));
// → "the quick brown fox jumps over the lazy dog the quick brown fox ju..."

// Inspect the stationary distribution
const dist = JSON.parse(chain.stationaryDistributionJson());
console.log(dist);
```

## API

| Method | Description |
|--------|-------------|
| `new WasmMarkovChain(order, smoothing, states)` | Create a chain. `states = []` to auto-discover from training. |
| `train(sequence: string[])` | Train on a sequence of states. Accumulates across calls. |
| `trainString(text: string)` | Convenience: treat each character as a state. |
| `nextState(current: string): string` | Sample one transition. Throws on unknown state. |
| `generate(start: string, length: number): string[]` | Emit exactly `length` states. |
| `generateString(seed: string, length: number): string` | Emit exactly `length` characters. |
| `probability(from: string, to: string): number` | `T[from][to]`, 0.0 if unknown. |
| `stationaryDistributionJson(): string` | JSON object `{ state: probability, … }`. |
| `states(): string[]` | Sorted list of all known states. |
| `transitionMatrixJson(): string` | JSON object of the full transition matrix. |

## Building the .wasm file

```bash
wasm-pack build --target web --out-dir pkg
```

This requires the `wasm32-unknown-unknown` Rust target:

```bash
rustup target add wasm32-unknown-unknown
```
