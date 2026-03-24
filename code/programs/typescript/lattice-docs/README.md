# Lattice Docs

Interactive documentation and live playground for the [Lattice](../../specs/17-lattice-transpiler.md) CSS superset language.

**Live at:** https://adhithyan15.github.io/coding-adventures/lattice/

## What is Lattice?

Lattice extends CSS with five features that compile away to plain CSS at build time:

- **Variables** — `$brand: #4a90d9;`
- **Mixins** — `@mixin button($bg) { ... }` / `@include button(red);`
- **Control Flow** — `@if`, `@for`, `@each`
- **Functions** — `@function spacing($n) { @return $n * 8px; }`
- **Modules** — `@use "colors";`

## Features

- Live playground with real-time Lattice → CSS compilation
- Full syntax reference with examples for every feature
- 3-pass compiler diagram
- Per-language install guide (Python, Go, Ruby, TypeScript, Rust, Elixir)
- Dark / light mode toggle

## Architecture

The docs app runs the full Lattice transpiler in the browser:

```
lattice.tokens  ──(?raw Vite import)──► string constant
lattice.grammar ──(?raw Vite import)──► string constant
                                              │
                                    parseTokenGrammar /
                                    parseParserGrammar
                                              │
                                    GrammarLexer + GrammarParser
                                              │
                                    LatticeTransformer (3-pass)
                                              │
                                    CSSEmitter
                                              │
                                         CSS output
```

Grammar files are inlined at Vite build time via `?raw` imports — no
server-side code and no network requests at runtime.

## Development

```bash
# Install all dependencies (in topological order)
cd ../../../packages/typescript/state-machine && npm install
cd ../directed-graph && npm install
cd ../grammar-tools && npm install
cd ../lexer && npm install
cd ../parser && npm install
cd ../lattice-lexer && npm install
cd ../lattice-parser && npm install
cd ../lattice-ast-to-css && npm install
cd ../lattice-transpiler && npm install
cd -

npm install
npm run dev
```

Then open http://localhost:5173/coding-adventures/lattice/

## Deploy

Automatic deploy on push to `main` via `.github/workflows/deploy-lattice-docs.yml`.
