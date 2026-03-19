# TypeScript Patterns Used in This Project

This document explains the TypeScript language features and patterns used
throughout the coding-adventures TypeScript packages.

## ESM Modules — Modern JavaScript Imports

All TypeScript packages use ES Modules (ESM), not CommonJS:

```json
{
  "type": "module"
}
```

```typescript
// Named exports
export function andGate(a: number, b: number): number {
    return a & b;
}

// Named imports
import { andGate, orGate } from './gates.js';

// Re-exports from index.ts
export { HalfAdder, FullAdder } from './adders.js';
export { ALU, ALUOp, ALUResult } from './alu.js';
```

Note the `.js` extension in imports — TypeScript requires it for ESM
even though the source files are `.ts`. This is because the import paths
must match the compiled output.

**Where used:** Every TypeScript package

## Interfaces — Structural Type Contracts

TypeScript interfaces define the shape of objects:

```typescript
interface Token {
    type: string;
    value: string;
    line: number;
    column: number;
}

interface BranchPredictor {
    predict(address: number): boolean;
    update(address: number, taken: boolean): void;
}
```

Like Go, TypeScript uses structural typing — any object with the right
shape satisfies the interface without explicit declaration.

**Where used:** Every TypeScript package

## Classes — Object-Oriented Patterns

TypeScript classes with access modifiers:

```typescript
export class ALU {
    private readonly bitWidth: number;

    constructor(bitWidth: number = 8) {
        this.bitWidth = bitWidth;
    }

    execute(op: ALUOp, a: number[], b: number[]): ALUResult {
        // ...
    }
}
```

- `private` — only accessible within the class
- `readonly` — set once in constructor, never modified
- `public` (default) — accessible everywhere

**Where used:** `code/packages/typescript/arithmetic/`, `code/packages/typescript/cpu-simulator/`

## Enums — Named Constants

```typescript
export enum ALUOp {
    ADD = 'add',
    SUB = 'sub',
    AND = 'and',
    OR  = 'or',
    XOR = 'xor',
    NOT = 'not',
}
```

String enums are preferred over numeric enums because they're
self-documenting in debug output and JSON serialization.

**Where used:** `code/packages/typescript/arithmetic/`, `code/packages/typescript/bytecode-compiler/`

## Union Types and Type Guards

TypeScript's union types are more powerful than enums for some cases:

```typescript
type NodeType = 'literal' | 'binary_op' | 'assignment' | 'if_stmt';

interface ASTNode {
    type: NodeType;
    children: ASTNode[];
}

// Type guard narrows the type
function isLiteral(node: ASTNode): node is LiteralNode {
    return node.type === 'literal';
}

if (isLiteral(node)) {
    // TypeScript knows node is LiteralNode here
    console.log(node.value);
}
```

**Where used:** `code/packages/typescript/parser/`, `code/packages/typescript/bytecode-compiler/`

## Generics — Reusable Type-Safe Code

```typescript
export class DirectedGraph<T> {
    private adjacency: Map<T, Set<T>> = new Map();

    addNode(node: T): void {
        if (!this.adjacency.has(node)) {
            this.adjacency.set(node, new Set());
        }
    }

    addEdge(from: T, to: T): void {
        this.addNode(from);
        this.addNode(to);
        this.adjacency.get(from)!.add(to);
    }

    topologicalSort(): T[] {
        // Kahn's algorithm...
    }
}
```

The `<T>` parameter means the graph works with any type — strings,
numbers, objects — while maintaining type safety.

**Where used:** `code/packages/typescript/directed-graph/`

## Strict Null Checks

With `strict: true` in tsconfig, `null` and `undefined` are explicit:

```typescript
function getBuildFile(directory: string): string | null {
    const buildPath = path.join(directory, 'BUILD');
    if (fs.existsSync(buildPath)) {
        return buildPath;
    }
    return null;  // must be explicit
}

const file = getBuildFile(dir);
if (file !== null) {
    // TypeScript knows file is string here (not null)
    const contents = fs.readFileSync(file, 'utf-8');
}
```

**Where used:** Every TypeScript package

## 32-Bit Integer Gotcha in Bitwise Operations

JavaScript (and TypeScript) bitwise operators work on signed 32-bit
integers. This causes surprising behavior:

```typescript
// Left shift wraps at 32 bits
1 << 32  // => 1 (not 4294967296!) — same as 1 << 0

// Bitwise OR produces signed 32-bit result
0xFFFFFFFF | 0  // => -1 (not 4294967295!)

// Fix: use unsigned right shift to convert to unsigned
(0xFFFFFFFF | 0) >>> 0  // => 4294967295
```

The `>>> 0` trick converts a signed 32-bit integer to unsigned. This is
critical in the logic-gates and arithmetic packages where bit manipulation
must produce unsigned results.

**Where used:** `code/packages/typescript/logic-gates/`, `code/packages/typescript/arithmetic/`

## Vitest — Modern Test Runner

TypeScript packages use Vitest (fast, ESM-native test runner):

```typescript
import { describe, it, expect } from 'vitest';
import { halfAdder } from '../src/adders.js';

describe('halfAdder', () => {
    it('should compute 0 + 0 = 0 with no carry', () => {
        const [sum, carry] = halfAdder(0, 0);
        expect(sum).toBe(0);
        expect(carry).toBe(0);
    });

    it('should compute 1 + 1 = 0 with carry', () => {
        const [sum, carry] = halfAdder(1, 1);
        expect(sum).toBe(0);
        expect(carry).toBe(1);
    });
});
```

**Where used:** Every TypeScript package's `tests/` directory

## package.json — Package Configuration

```json
{
  "name": "coding-adventures-arithmetic",
  "version": "0.1.0",
  "type": "module",
  "main": "src/index.ts",
  "scripts": {
    "test": "vitest run",
    "build": "tsc"
  },
  "dependencies": {
    "coding-adventures-logic-gates": "file:../logic-gates"
  },
  "devDependencies": {
    "vitest": "^3.0.0",
    "typescript": "^5.0.0"
  }
}
```

Key points:
- `"main": "src/index.ts"` — points to source for Vitest resolution
- `"file:../logic-gates"` — local path dependencies for development
- `"type": "module"` — enables ESM imports

**Where used:** Every TypeScript package

## `readonly` Arrays and Tuples

TypeScript can enforce immutability at the type level:

```typescript
function processGates(gates: readonly number[]): number {
    // gates.push(1);  // Compile error! readonly
    return gates.reduce((a, b) => a & b, 1);
}

// Tuple types for fixed-size returns
function halfAdder(a: number, b: number): [number, number] {
    return [a ^ b, a & b];  // [sum, carry]
}
```

**Where used:** Various packages for safety
