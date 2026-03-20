# @coding-adventures/ca-capability-analyzer

Static analyzer that walks TypeScript/JavaScript ASTs to detect OS capability usage and compares against declared capabilities.

## What It Does

This package answers three questions about your code:

1. **What OS capabilities does this code use?** (filesystem, network, process execution, environment variables, FFI)
2. **Does the code contain banned constructs?** (eval, dynamic require, Reflect.apply, etc.)
3. **Does actual usage match declared capabilities?** (comparing against a `required_capabilities.json` manifest)

## How It Works

The analyzer uses the TypeScript Compiler API to parse source files into ASTs, then walks those trees to detect:

- **Import-level capabilities**: `import fs from "fs"` implies filesystem access
- **Call-level capabilities**: `fs.readFileSync("/etc/passwd")` is specifically a filesystem read of `/etc/passwd`
- **Environment access**: `process.env.SECRET_KEY` reads an environment variable
- **Network access**: `fetch(url)`, `http.request(...)`, etc.
- **Banned constructs**: `eval(...)`, `new Function(...)`, dynamic `require()`, `Reflect.apply()`

## Capability Notation

Capabilities use a three-part `category:action:target` notation:

| Example | Meaning |
|---------|---------|
| `fs:read:/etc/passwd` | Reading a specific file |
| `fs:write:*` | Writing to an unknown file |
| `net:connect:*` | Network connection |
| `proc:exec:*` | Spawning a child process |
| `env:read:HOME` | Reading the HOME env var |
| `ffi:*:*` | Foreign function interface |

## Usage

### As a Library

```typescript
import { analyzeSource, parseManifest, compareCapabilities } from "@coding-adventures/ca-capability-analyzer";

const result = analyzeSource('import fs from "fs"; fs.readFileSync("/data");', "app.ts");
console.log(result.capabilities);
console.log(result.banned);

const manifest = parseManifest(fs.readFileSync("required_capabilities.json", "utf-8"));
const comparison = compareCapabilities(result.capabilities, manifest);
console.log(comparison.undeclared);  // Security concern!
```

### As a CLI

```bash
# Detect all capabilities
npx ca-capability-analyzer detect src/**/*.ts

# Find banned constructs
npx ca-capability-analyzer banned src/**/*.ts

# Compare against manifest
npx ca-capability-analyzer check required_capabilities.json src/**/*.ts
```

## Design Decision

For banned construct detection, we delegate to ESLint where possible (`no-eval`, `no-implied-eval`, `no-new-func`). Our analyzer focuses on **capability detection** (mapping imports/calls to `category:action:target`) and **manifest comparison**.

## Dependencies

- **Runtime**: TypeScript Compiler API (for AST parsing)
- **Dev**: vitest (testing), @types/node

Zero external runtime dependencies beyond the TypeScript compiler.
