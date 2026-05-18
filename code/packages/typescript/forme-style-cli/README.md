# @coding-adventures/forme-style-cli

Node.js CLI for the **Forme Style IR** family — wraps
[`forme-style-orchestrator`](../forme-style-orchestrator) with file
I/O so the FM04 family is usable from a shell without writing any
TypeScript glue.

Per [FM03 §5 (CLI surface)](../../specs/FM03-forme-orchestrator.md).
Seventh package of the FM04 family — the first one with non-empty
`required_capabilities` (`["fs"]`).

## Install / use

```bash
# from this monorepo
npm install -g ./code/packages/typescript/forme-style-cli/
forme-style --help
```

Or run programmatically:

```ts
import { run } from "@coding-adventures/forme-style-cli";
import { promises as fs } from "node:fs";

const code = await run(["doc.json", "--target", "css"], {
  stdout: process.stdout,
  stderr: process.stderr,
  readFile: (p) => fs.readFile(p, "utf8"),
  writeFile: (p, c) => fs.writeFile(p, c, "utf8"),
  readStdin: () => Promise.resolve(""),
});
process.exit(code);
```

## CLI surface

```
forme-style <doc.json> --target <css|latex|terminal> [options]
forme-style - --target <css|latex|terminal> [options]      # stdin
forme-style --help

Options:
  --target <css|latex|terminal>   Required.
  --theme <name>                  Apply theme by name (requires --themes).
  --themes <themes.json>          Load registry from JSON file
                                  { "themes": [ { "name": ..., ... }, ... ] }.
  --active <ctx,ctx,...>          Comma-separated active contexts (default: empty).
  --used <id,id,...>              Per-page slice — emit only these rule ids.
  --scope <string>                Caller-trusted scope prefix.
  --out <file>                    Write to file (default: stdout).
  --help                          Print usage and exit.
```

## Exit codes

| Code | Meaning                                         |
|------|-------------------------------------------------|
| 0    | success                                         |
| 1    | validator rejected the input (`StyleError`)     |
| 2    | file I/O or argument-parse error                |
| 3    | reserved (unknown target; caught by arg parser) |

## Examples

```bash
# CSS to stdout
forme-style doc.json --target css

# LaTeX to a file
forme-style doc.json --target latex --out preamble.tex

# Pipe a doc via stdin, slice to one rule, terminal output
cat doc.json | forme-style - --target terminal --used body

# Apply a theme
forme-style doc.json --target css --themes themes.json --theme dark

# Per-page CSS slicing (FM06 integration)
forme-style doc.json --target css --active screen \
  --used body,heading-1 --scope "#page-abc123" --out per-page.css
```

## Themes file format

```json
{
  "themes": [
    {
      "name": "dark",
      "tokens": {
        "colors": { "text": { "kind": "rgb", "r": 240, "g": 240, "b": 240 } }
      }
    },
    {
      "name": "high-contrast",
      "tokens": { ... }
    }
  ]
}
```

Malformed individual theme entries (missing `name`, forbidden
proto-pollution names, non-object) are **silently skipped** so a
single bad theme in a larger file doesn't take down the whole run.
Top-level shape violations (`themes` not an array; payload not an
object) exit 2.

## Architecture

```
                 ┌──────────────┐
       argv ───→ │  parseArgs   │ → ParsedArgs
                 └──────────────┘
                        │
                        ▼
                 ┌──────────────┐
   stdin / fs ─→ │  read input  │ → StyleDocument JSON
                 └──────────────┘
                        │
                        ▼
                 ┌──────────────┐
                 │ load themes  │ ← (optional) themes.json
                 └──────────────┘
                        │
                        ▼
                 ┌──────────────┐
                 │   compile()  │ ← orchestrator: validate → theme → dispatch
                 └──────────────┘
                        │
                        ▼
                 ┌──────────────┐
                 │ write output │ → stdout or --out file
                 └──────────────┘
```

All real logic lives in `run(argv, io)` in `src/cli.ts`.  The npm
`bin` shim (`src/bin.ts`) is a 30-line wrapper that wires
`process.stdin` / `process.stdout` / `node:fs` to the testable
function.  Tests exercise `run` directly with in-memory I/O — no
subprocess spawning.

## Capabilities — `["fs"]`

First FM04-family package with a non-empty capability set.  Reads
the positional input file (or stdin via `-`), reads the optional
`--themes` file, writes to `--out` or stdout.  No network, no
shell, no env reads beyond `process.argv`.

## Security posture

Three concerns explicitly addressed:

1. **Untrusted JSON input.**  `JSON.parse` failure on either the
   document or the themes file exits 2 with the error message
   captured (never thrown to the user's terminal as a stack trace).
2. **Theme registry from untrusted file.**  Pass-through to
   `forme-style-theme`'s `register`, which itself refuses forbidden
   names (`__proto__` / `constructor` / `prototype`).  Malformed
   theme entries are silently skipped per file-level resilience.
3. **Path handling.**  File paths are passed verbatim to
   `io.readFile` / `io.writeFile` — the production wiring uses
   `fs.promises.readFile` / `writeFile` which honour OS-level
   permission checks.  The CLI doesn't perform any path
   normalisation or sandboxing; callers running the binary from
   untrusted shells should restrict the working directory via
   standard OS mechanisms.

## Tests

30 tests in `cli.test.ts`:

- Help / usage / no-args / `-h` alias
- Happy-path dispatch (CSS / LaTeX / terminal)
- Stdin / stdout / `--out` file streaming
- Themes (load + named apply; missing → warn; malformed → exit 2;
  partial corruption → skip-and-continue)
- `--active` / `--used` / `--scope` pass-through
- Every exit-code path (validator fail; missing input; invalid JSON;
  missing flag value; unknown flag; too many positional args; write
  failure)
- Warning propagation (translator warnings → stderr, exit still 0)

Coverage: **97.12% line / 95.78% branch** — above the FM04 §14.4
≥95% line target.  Uncovered lines are the orchestrator's
"unexpected exception from `compile`" defensive catch (unreachable
via the orchestrator's documented contract) and the trailing-
newline branch for non-terminal output.

## Spec adherence

Implements FM03 §5 CLI surface verbatim.  No spec divergences.

## v0 simplifications

- **No `--watch` mode.**  Add when the dev server (forme-dev-server)
  needs it.
- **No `--config` flag for global defaults.**  Add when there's a
  documented config file format.
