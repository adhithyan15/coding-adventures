# @coding-adventures/forme-errors

Forme kernel error model — three error classes plus the canonical code vocabulary every stage uses.

See [code/specs/FM01-forme-kernel.md](../../../specs/FM01-forme-kernel.md) §6 for the full design.

## Exports

| Class / Constant       | Purpose                                                                  |
| ---------------------- | ------------------------------------------------------------------------ |
| `StageError`           | Typed error a stage throws. Carries provenance + structured fields.       |
| `CapabilityError`      | Subclass; code locked to `CAPABILITY_DENIED`, `recoverable: false`.       |
| `CancellationError`    | Propagated for cancellation. **Not** a StageError, by design.            |
| `isCancellationError`  | Cross-realm duck-typing predicate (Worker-safe).                         |
| `ERROR_CODES`          | Frozen vocabulary of kernel-blessed codes.                                |
| `KernelErrorCode`      | String-literal union of every `ERROR_CODES` value.                       |

## Usage

```typescript
import {
  StageError, CapabilityError, CancellationError,
  ERROR_CODES,
} from "@coding-adventures/forme-errors";

// A parse failure on a specific input.
throw new StageError({
  code:        ERROR_CODES.PARSE_ERROR,
  message:     "Invalid frontmatter: unclosed delimiter",
  inputPath:   source.path,
  inputId:     source.identity,
  stageName:   "@forme/parse-markdown",
  recoverable: true,                // best-effort mode may skip this input
  fields:      { line: 3 },
});

// Capability gate.
if (!hasCapability("network:api.github.com")) {
  throw new CapabilityError({
    message:    "Stage attempted to fetch GitHub without `network:api.github.com`",
    capability: "network:api.github.com",
    stageName:  "@forme/source-github",
  });
}

// Cancellation, normally thrown by `ctx.cancellation.throwIfCancelled()`.
throw new CancellationError("user pressed Ctrl-C");
```

## Why `CancellationError` is not a `StageError`

Cancellation is an orchestrator-level event, not a stage-level failure. The orchestrator's error boundary should let it unwind cleanly — wrapping it in `UNCAUGHT` and triggering retry/fallback machinery would be wrong. Splitting the type at the kernel makes that the default behaviour rather than something every error boundary has to remember.

## `toJson()` shape

`StageError.toJson()` returns a stable JSON-safe object suitable for structured logs, telemetry events, and editor IPC:

```json
{
  "name":        "StageError",
  "code":        "PARSE_ERROR",
  "message":     "Invalid frontmatter",
  "inputPath":   "posts/draft.md",
  "inputId":     "01952c0d-7e63-7000-8000-...",
  "stageName":   "@forme/parse-markdown",
  "recoverable": true,
  "fields":      { "line": 3 },
  "cause":       null
}
```

`cause` is reduced to `String(cause)` because arbitrary thrown values aren't generally JSON-safe; richer structured data goes in `fields`.

`CapabilityError.toJson()` adds a `capability` field on top of this shape.

## Coverage

```bash
npm install
npx vitest run --coverage
```

Targets: 100% line + branch on every executable file.
