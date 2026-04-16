# GE-225 Simulator (TypeScript)

Behavioral TypeScript simulator for the **GE-225 instruction repertoire**.

This package mirrors the Python GE-225 simulator closely enough to serve as a
second implementation for backend validation work.

## Scope

- 20-bit word-addressed memory
- documented GE-225 memory-reference and fixed-word instruction families
- index-group state, compare/skip behavior, and host-side console helpers
- focused tests for arithmetic, branching, block move, and typewriter flow

## Running Tests

```bash
npm install
npx vitest run --coverage
```
