# @coding-adventures/ca-capability-analyzer

Static capability analyzer and VER15 CWE detector for TypeScript and JavaScript packages (Spec 13 Layer 3 & 4, VER08 / VER15). Uses only the `typescript` compiler API; zero other third-party runtime dependencies.

## Overview

Every package in this monorepo declares its operating system capability requirements in a `required_capabilities.json` manifest. Pure computation packages declare an empty list (`capabilities: []`), enforcing a deny-by-default execution environment.

`ca-capability-analyzer` statically inspects TypeScript and JavaScript source code to:
1. Detect ambient OS capabilities:
   - `fs:read`: filesystem reads (`readFile`, `readdir`, `stat`, `open`, etc.)
   - `fs:write`: filesystem writes (`writeFile`, `appendFile`, `mkdir`, `unlink`, etc.)
   - `net:socket`: network connections and HTTP (`fetch`, `WebSocket`, `net.connect`, etc.)
   - `process:spawn`: process creation (`spawn`, `exec`, `execFile`, `fork`)
   - `env:read`: environment access (`process.env`, `Deno.env`, `Bun.env`)
   - `ffi:unsafe`: foreign function calls (`process.dlopen`, `ffi-napi`, `bun:ffi`)
   - `time:clock`: system clock checks (`Date.now`, `new Date()`, `performance.now`)
   - `random:entropy`: entropy generation (`Math.random`, `crypto.getRandomValues`)
2. Detect VER15 Common Weakness Enumeration (CWE) patterns:
   - **CWE-1321**: Prototype pollution (`__proto__`, `constructor`, `prototype`, dynamic bracket writes on plain objects)
   - **CWE-1333**: Inefficient regular expressions / ReDoS (catastrophic backtracking shapes, dynamic unvalidated RegExp)
   - **CWE-674**: Uncontrolled recursion (recursive calls lacking depth limits or step budgets)
   - **CWE-94 / CWE-78**: Code and OS command injection (`eval`, `new Function`, dynamic `child_process.exec`)
3. Cross-reference detected capabilities against `required_capabilities.json` in `--strict` mode.

## CLI Usage

```bash
# Analyze a package directory
npx ca-capability-analyzer --dir code/packages/typescript/note-frequency

# Strict mode: fail with exit code 1 on any undeclared capability or unexempt CWE
npx ca-capability-analyzer --dir code/packages/typescript/note-frequency --strict

# Machine-readable JSON output
npx ca-capability-analyzer --dir code/packages/typescript/note-frequency --json
```

## Exit Codes

| Code | Meaning |
|---|---|
| `0` | Pass: all detected capabilities are declared, no unexempt CWEs |
| `1` | Violations found: undeclared capability or unexempt CWE in `--strict` mode |
| `2` | Error: invalid arguments, file read failure, or malformed JSON |
