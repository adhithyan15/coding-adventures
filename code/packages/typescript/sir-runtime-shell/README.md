# @coding-adventures/sir-runtime-shell

Shell runtime for **Semantic-IR-emitted TypeScript/JavaScript**.

The Ruby→SIR frontend lowers a backtick literal `` `cmd` `` to
`BuiltinCall("backtick", [StrLit cmd])`. Emitted TypeScript needs a runtime
landing point for that builtin — one that runs the command through the system
shell and returns its stdout, exactly as Ruby's backtick expression does. That
runtime is this package.

## Where it fits in the stack

```
Ruby source ─▶ ruby-to-semantic-ir ─▶ Semantic IR ─▶ semantic-ir-to-typescript ─▶ .ts
                                                                                  │ imports
                                                                                  ▼
                                                              @coding-adventures/sir-runtime-shell
```

The TypeScript backend emits an import of this package only when a module uses a
backtick; pure modules never gain the dependency.

## What it does — Ruby backtick semantics

In Ruby, `` `cmd` `` (and `%x{cmd}`) hands the whole command line to the system
shell, waits for it, and evaluates to the command's **standard output** as a
string. The child's exit status is recorded in `$?` but does *not* change the
value — even a non-zero exit returns whatever was printed to stdout. Standard
error is not captured by the expression. This package mirrors all of that with
Node's built-in `node:child_process` (no third-party dependencies):

| Ruby backtick behaviour | TypeScript implementation |
|---|---|
| runs via the system shell | `execSync` (spawns through the shell) |
| returns captured stdout as a string | `{ encoding: "utf8" }` → return value |
| ignores the child's exit status | `catch` the non-zero exit, return its `stdout` |
| stderr goes to the parent | not included in the returned value |

## Security — running via the shell is intentional and author-supplied input only

`execSync` runs the command *through the system shell*. This is **load-bearing**:
Ruby backtick is *defined* as "run via the shell", so shell features (pipes,
redirections, globbing, `$VAR`) are part of the builtin. Running without a shell
would silently change the meaning of every compiled backtick.

There is no new untrusted-input path. `command` is the string literal the
programmer wrote inside the backticks of their *own* Ruby source, threaded
verbatim through the compiler into the emitted TypeScript. It carries exactly the
trust level Ruby itself grants it — the author's own code — and this package
interpolates **no** external or runtime-derived data into the command.

## API

| Export | Purpose |
|---|---|
| `backtick(command): string` | Run `command` via the system shell and return its captured stdout (Ruby `` `cmd` ``). Non-zero exits still return stdout. |
| `Val` | The universal SIR value type alias (`any`) at this boundary. |

## Usage

```ts
import { backtick } from "@coding-adventures/sir-runtime-shell";

backtick(`"${process.execPath}" -e "process.stdout.write('123')"`); // "123"
backtick(`"${process.execPath}" -e "process.exit(1)"`);             // ""  (non-zero exit → stdout still returned)
```

## Development

```bash
npm ci
npx tsc --noEmit      # strict typecheck
npx vitest run --coverage
```

## License

MIT
