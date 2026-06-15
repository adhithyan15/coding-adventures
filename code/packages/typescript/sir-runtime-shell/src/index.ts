/**
 * Shell-out — the SIR `backtick` builtin (Ruby `` `cmd` `` on Node `child_process`).
 *
 * The Ruby→SIR frontend lowers a backtick literal `` `cmd` `` to
 * `BuiltinCall("backtick", [StrLit cmd])`. This module is the
 * *TypeScript/JavaScript* landing point for that builtin: it runs the command
 * through the system shell and returns the command's captured standard output as
 * a string, exactly as Ruby's backtick expression does.
 *
 * **What Ruby backtick does — the contract we mirror.** In Ruby, `` `cmd` `` (and
 * its twin `%x{cmd}`) hands the *whole command line* to the system shell — on a
 * POSIX host that is `/bin/sh -c cmd` — waits for it to finish, and evaluates to
 * the command's standard output as a string. The child's exit status is recorded
 * in `$?` but does **not** affect the value: even a command that exits non-zero
 * yields whatever it printed to stdout (often the empty string). Standard error
 * is not captured by the expression. We reproduce all of this below.
 *
 * | Ruby backtick behaviour          | TypeScript implementation                     |
 * |----------------------------------|-----------------------------------------------|
 * | runs via the system shell        | `execSync` (spawns through the shell)         |
 * | returns captured stdout as a str | `{ encoding: "utf8" }` → return value         |
 * | ignores the child's exit status  | `catch` non-zero exit, return its `stdout`    |
 * | stderr goes to the parent        | not included in the returned value            |
 *
 * **SECURITY — running via the shell is intentional and load-bearing.**
 * `execSync` runs the command *through the system shell* (`/bin/sh -c` on POSIX,
 * `cmd.exe /c` on Windows). In most Node code, building a shell command from
 * *untrusted runtime input* is a shell-injection red flag. Here it is the
 * opposite — it is *required* for faithful semantics, and there is no new
 * untrusted-input path:
 *
 * - **Ruby backtick is defined as "run via the shell."** Ruby always routes
 *   `` `cmd` `` through `/bin/sh -c`. Shell metacharacters (pipes `|`,
 *   redirections `>`, globbing `*`, `$VAR` expansion, `;` sequencing) are part of
 *   the feature. Running the command without a shell would silently change the
 *   meaning of every compiled Ruby program that uses a backtick.
 * - **The command is author-supplied, not attacker-supplied.** `command` is the
 *   string literal the programmer wrote *inside the backticks of their own Ruby
 *   source*, threaded verbatim through the compiler into the emitted TypeScript.
 *   It carries exactly the trust level it had in the original Ruby program — the
 *   author's own code — which is precisely the trust level Ruby itself grants it.
 *   This package interpolates **no** external or runtime-derived data into the
 *   command, so it introduces no new injection surface.
 */

import { execSync } from "node:child_process";

/** The SIR universal value type at this package's boundary. */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type Val = any;

/**
 * Run `command` via the system shell and return its captured stdout.
 *
 * This is the runtime for the SIR `backtick` builtin, modelling Ruby's
 * `` `cmd` `` expression. The command is passed to the system shell (`/bin/sh -c`
 * on POSIX, `cmd.exe /c` on Windows) via {@link execSync}, run to completion, and
 * its standard output returned as a UTF-8 string.
 *
 * The child's exit status is **ignored**: like Ruby, a command that exits
 * non-zero still returns whatever it wrote to stdout (which may be the empty
 * string). `execSync` *throws* on a non-zero exit, so we catch the error and
 * recover its captured `stdout` — falling back to `""` when none is available.
 * Standard error is not part of the returned value, mirroring Ruby, where the
 * backtick value is stdout only.
 *
 * See the module doc comment for the full Ruby↔TS mapping table and the SECURITY
 * note explaining why running via the shell is intentional and safe here (the
 * command is author-supplied from the compiled program's own source, exactly as
 * in Ruby; no untrusted runtime input is interpolated).
 */
export function backtick(command: string): string {
  try {
    // execSync runs the command through the system shell, matching Ruby
    // backtick; encoding:"utf8" makes it return a string rather than a Buffer.
    return execSync(command, { encoding: "utf8" });
  } catch (err: unknown) {
    // A non-zero exit makes execSync throw. Ruby returns stdout regardless of
    // $?, so recover the captured stdout from the thrown error. The error is an
    // ExecSyncError whose `stdout` may be a string or Buffer; narrow defensively
    // and coerce to a string, defaulting to "" when no stdout was captured.
    const stdout = (err as { stdout?: Buffer | string | null }).stdout;
    return stdout?.toString() ?? "";
  }
}
