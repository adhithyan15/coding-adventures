# B05 — Windows BUILD Executor: PowerShell + BUILD_windows for Starlark

## Status

Planned. Tracked by issue [#787](https://github.com/adhithyan15/coding-adventures/issues/787).
This spec consolidates the design rationale so a future PR can implement it
deliberately rather than reviving the stale branches that have already been
closed (#355, #1523).

## Purpose

Two related defects in the Go build tool's Windows path force packages to
work around the build system instead of relying on it. Both are fixable
together because they share the same execution surface.

1. **`cmd.exe` strips outer quotes from arguments** before handing them
   to the child process. This corrupts paths in commands the rest of the
   repo treats as portable.
2. **Starlark BUILD files bypass `BUILD_windows` overrides** in the
   plan-loading path, so a package that ships a Windows-specific shell
   override in addition to a `BUILD.lark` config gets the wrong commands
   on Windows.

This spec defines the target behavior and the migration plan. The
implementation lives in `code/programs/go/build-tool/internal/executor/`
and `code/programs/go/build-tool/main.go`.

---

## §1 Problem 1 — `cmd.exe` argument mangling

### What's on main today

`code/programs/go/build-tool/internal/executor/executor.go` runs every
BUILD command through:

```go
return exec.Command("cmd", "/C", command)
```

The Unix counterpart is `sh -c command`. The asymmetry hides a real
behavioral difference: `cmd.exe` mangles its argument list before any
process is spawned.

### Why it's broken

`cmd.exe` has a 35-year-old convention where it strips the outermost
double-quote pair from each argument before invoking the child program.
That collides with the rest of the repo, where BUILD files routinely
contain quoted relative paths:

```bash
uv pip install -e "../../../packages/python/foo" --quiet
```

When this string is handed to `cmd /C`, the outer quotes around the path
are removed before `uv` ever sees them. `uv` then receives the literal
string `../../../packages/python/foo`, but with surrounding context that
makes it interpret characters like `.` and `/` differently. In the worst
case the trailing `"` is URL-encoded to `%22`, producing an invalid
path that fails with **No such file or directory**.

The same class of bug bites any tool that:

- expects quoted strings to round-trip verbatim, or
- builds its own command-line splitter (Python's `subprocess`,
  Node's `child_process`, etc.) that disagrees with `cmd.exe`'s rules.

The reason this is invisible on macOS and Linux is that `sh -c` is a
real shell with consistent quoting rules. There is no comparable
shell-quoting layer between `cmd /C` and the child process.

### Target behavior

On Windows, the executor must invoke commands through PowerShell 7
(`pwsh`) instead of `cmd.exe`:

```go
return exec.Command("pwsh", "-Command", command)
```

`pwsh` is the right replacement because:

- It is **pre-installed** on every GitHub Actions Windows runner.
- It handles double-quoted strings the same way `bash` does: quotes are
  passed through to the child process, not stripped.
- It supports the **`&&` operator** (since PowerShell 7.0) for
  fail-fast command chaining, which is the same idiom Unix BUILD files
  use.
- It supports **forward-slash paths**: PowerShell normalizes `/` to `\`
  for Win32 API calls, so Unix-style relative paths work without
  modification.
- Pipes (`|`) and the common redirection forms (`>`, `>>`, `2>&1`)
  behave the way BUILD authors expect.

`powershell.exe` (the legacy 5.1 ISE shipped with Windows) does **not**
work as a replacement: it lacks `&&` and has a different parameter
parser that breaks BUILD commands with embedded `=` signs.

### What this is *not*

- Not a way to run PowerShell scripts as BUILD files. BUILD files
  remain plain shell strings; we only change the shell that interprets
  them on Windows.
- Not a generic script interpreter. The executor still treats the BUILD
  file's content as one command-line string and hands it to one shell
  invocation. Multi-statement scripts that work today on Unix continue
  to work on Windows because `pwsh -Command` handles `&&`, `;`,
  `;`-separated lines, and embedded newlines.
- Not a Windows-only escape hatch. If a package needs Windows-specific
  commands, the existing `BUILD_windows` mechanism remains the answer
  — but it can finally be relied upon (see §2).

---

## §2 Problem 2 — Starlark packages skip `BUILD_windows`

### What's on main today

The plan-loading path in `code/programs/go/build-tool/main.go` looks
roughly like this (simplified):

```go
if pkg.IsStarlark {
    // Use the BUILD.lark-evaluated command list.
    return planFromStarlark(pkg)
}

// Otherwise, look for BUILD_windows on Windows, BUILD on Unix.
return planFromShellBuildFile(pkg, runtimeOS)
```

### Why it's broken

The condition is too coarse. A package can legitimately have:

- `BUILD.lark` — the canonical Starlark configuration, used everywhere
- `BUILD_windows` — a plain shell override that handles a Windows-only
  detail (a different toolchain invocation, a path translation, a
  conditional command)

The current code uses the Starlark plan unconditionally when
`IsStarlark` is true, so the `BUILD_windows` override is silently
ignored. The package author has done the right thing by providing a
platform escape hatch, and the build tool drops it on the floor.

### Target behavior

The plan-loading path must honor `BUILD_windows` before falling back to
Starlark. The intended precedence on Windows is:

1. `BUILD_windows` (plain shell), if present
2. `BUILD.lark` (Starlark), if present and §1 above doesn't apply
3. `BUILD` (plain shell)

On non-Windows runs the precedence is `BUILD.lark` before `BUILD`, with
no `BUILD_windows` consultation. The change is local to the plan loader
— neither the Starlark VM (B01) nor the sandbox (B02) needs to know.

```go
if runtimeOS == "windows" {
    if hasBuildWindows(pkg) {
        return planFromShellBuildFile(pkg, "BUILD_windows")
    }
}

if pkg.IsStarlark {
    return planFromStarlark(pkg)
}

return planFromShellBuildFile(pkg, "BUILD")
```

The same precedence applies to `BUILD_macos` and `BUILD_linux` if they
exist. They are rare today but the logic is identical.

---

## §3 Migration Plan

These are independent changes that can land in any order.

### Step 1 — switch executor to `pwsh`

One-file change to `code/programs/go/build-tool/internal/executor/executor.go`:

```go
// Replace
return exec.Command("cmd", "/C", command)
// With
return exec.Command("pwsh", "-Command", command)
```

Plus the docstring update describing the rationale (the comments above
the function should match this spec).

### Step 2 — fix plan loader

One change to `code/programs/go/build-tool/main.go`'s plan-loading path:
re-order the conditionals so `BUILD_windows` is consulted before
`IsStarlark` is checked.

### Step 3 — sweep BUILD files

After Step 1 lands, every existing `BUILD_windows` file becomes a
candidate for elimination. If a `BUILD_windows` exists only because
`cmd.exe` mangled the equivalent `BUILD`, it can now be deleted —
`pwsh` handles the same command. The sweep is best done as one PR per
language (Python, Ruby, Go, …) so each can be reviewed against the
matching toolchain.

A `BUILD_windows` should remain only when the Windows command is
genuinely different (e.g. invoking `cl.exe` instead of `gcc`,
or working around an MSVC linker conflict). The repo already has
notes on these in [`lessons.md`](../../lessons.md) under the Windows
toolchain entries.

### Step 4 — CI pin

`pwsh` is preinstalled on `windows-latest` runners, but the version
varies. CI should print `pwsh --version` early in the Windows job so
log output is unambiguous. Pin to `pwsh 7.2+` in the workflow if
version drift becomes a problem.

---

## §4 Test Plan

Each step has its own validation approach.

**Step 1 (executor):**

- Unit test in `executor_test.go` using a fake command that prints its
  argv: confirm quoted relative paths round-trip on Windows after the
  switch. The test currently exists for the Unix path; the Windows
  variant needs to be added.
- E2E: a minimal BUILD file that runs
  `uv pip install -e "../foo" --quiet` (or its equivalent for any
  language with quoted relative-path installs) compiles and installs
  cleanly on the Windows runner.

**Step 2 (plan loader):**

- Unit test in `main_test.go` (or whatever its plan-loading sibling is
  called): a fixture package with both `BUILD.lark` and `BUILD_windows`
  must produce the `BUILD_windows` plan when `runtimeOS == "windows"`
  and the Starlark plan elsewhere.
- E2E: construct a fixture package whose Starlark plan would fail on
  Windows and whose `BUILD_windows` would succeed; CI's Windows job
  must succeed.

**Steps 3–4** are observational: monitor CI for regressions over a few
PRs after each language sweep, then move to the next language.

---

## §5 Out of Scope

- **Native PowerShell BUILD files.** A future spec could allow
  `BUILD.ps1` for genuinely Windows-specific build logic, but that is
  not part of B05. B05 only changes the shell `cmd /C` runs in.
- **Migrating Unix executor off `sh -c`.** `bash`, `zsh`, and `dash`
  all behave compatibly with `sh -c`'s quoting rules; there's no
  equivalent motivation on Unix.
- **WSL.** Running Linux BUILD files inside WSL on Windows is a
  different system architecture. B05 stays on the Win32 PowerShell path.
- **Reviving any of the closed PRs (#355, #1523).** Those branches
  bundled too many other unrelated changes (Swift hello-world apps,
  unrelated Cargo.toml regressions, droplet.md edits) and the Windows
  executor work has since drifted from main. The implementation should
  be a fresh PR built on current main, focused on Steps 1 and 2.

---

## §6 References

- Issue [#787](https://github.com/adhithyan15/coding-adventures/issues/787)
  — the durable home for this work
- Closed PR #355 — first attempt; closed for being too stale
- Closed PR #1523 — second attempt; closed for the same reason plus
  bundling unrelated changes
- `B01-build-lark-format.md` — the Starlark BUILD format that Step 2
  needs to coexist with
- `B02-build-sandbox.md` — the execution sandbox the executor runs
  inside
- [PowerShell 7 — about_Operators](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_operators)
  — `&&` and `||` semantics
- [`cmd.exe` documentation](https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/cmd)
  — `/C`, `/D`, `/S` and the quote-stripping rule
