# "The workflow rebuilds it" was true of a different check

I left the rebuilt `pkg/engram_engine.wasm` out of a fix, reasoning that the
release workflow builds the wasm fresh and compares it against the fresh dist
copy, so a committed binary would be noise. That is true — of the release
workflow.

The crate's BUILD file runs `node js/smoke.mjs` against the **checked-in**
artifact, and its comment says why, citing the incident already in this file:

> That artifact went two months stale without a single failure — it still
> refused Anki import in the browser long after the source had stopped doing
> so, and the smoke test agreed with it because the assertions were written
> against the old behaviour too. Two stale things matching each other reads
> exactly like a passing test.

So CI failed on a branch where every local test passed, because locally I had
rebuilt the artifact and never committed it.

The sharper part: this also explains why `main` looked green while being
broken. Main has the new source, the OLD committed artifact, and the OLD smoke
assertions — artifact and test agree, so the BUILD check passes. Only the
release workflow, which builds fresh, disagreed. **The failure mode the BUILD
comment describes was live on main at the moment I read the comment.**

Generalising: before deciding a build output does not need committing, find
every check that consumes it. I checked one and generalised from it, and the
one I checked was the one that regenerates it.

### 2026-09-07 — Erlang eval quoting in Windows probes

A manual PowerShell `erl -eval` probe lost its embedded double quotes and
failed before evaluating the program. Use an argument-preserving subprocess
API for Erlang expressions. The Rust BEAM corpus runner passes a fixed eval
argument through `Command`, uses a private working directory, and checks both
process status and the complete integer output. All 21 programs passed.

### 2026-09-07 — Validate rebuild exit before reusing test binaries

The first LLVM input_more arm omitted its required Ok(()) result. The rebuild
failed, and an existing matrix binary still reproduced the old backend refusal.
Check the build exit and log before running a retained executable. The LLVM
builtin dispatcher returns Result, so new successful arms must return Ok(()).
PowerShell also does not support Bash brace-expanded path lists; pass explicit
paths to rg when finding the native builtin tables.

The next executable probe caught a missing state.env registration for the new
LLVM result. Emitting SSA text alone is insufficient: register the destination
just like input_i64 so the subsequent EOF comparison can resolve it.

A guarded test insertion expected tool_ok, but this matrix uses clang_ok. The
assertion prevented the edit; the following filter consequently ran zero tests.
Require a positive test count as well as exit success for focused validation.

### 2026-09-07 — Re-enumerate matrix rows after main advances

An ALGOL insertion shifted the FLOW-MATIC rows to 439–442. A positive row
sentinel proves that row ran, not that it still names the intended source.
Check row-to-source identity after syncing; row 439 reproduced the intended
WASM input_more whitelist refusal.

### 2026-09-07 — A conflict resolution with no markers left can still be structurally broken

Resolving a "keep both" rebase conflict in `engram-core-wasm/src/lib.rs`, I
concatenated the HEAD hunk and the branch hunk, then verified the resolution by
grepping for `<<<<<<<`, `=======`, and `>>>>>>>`. Zero matches, so I called it
resolved and ran `git rebase --continue`. The rebase succeeded; git does not
parse what it commits.

The conflict boundary had fallen **inside** an `assert!` call. HEAD's hunk ended
at the message string, and the `);` and `}` that closed the call and its test
function sat on the other side of the divider. Concatenating the two hunks left
main's last test unterminated, so `mod tests` never closed. The file was 6,500
lines short of balanced and had no markers anywhere.

`cargo build` found it immediately: "mismatched closing delimiter", pointing at
`mod tests {` thousands of lines above.

**A marker grep proves the conflict is resolved, not that the result is valid
code.** Markers are the only thing git guarantees it leaves behind; brace
balance, import order, and duplicated or dropped statements are all invisible to
it. Nothing short of compiling the crate and running its tests establishes that
a resolution preserved both sides.

So: after every conflict resolution, run the crate's build AND its test suite
before `rebase --continue` or `commit` -- and check the test **count** against
what each side contributed. Here the expected union was 83 (main) + 4 (branch) =
87; seeing 87 is what confirmed nothing was silently dropped, which a green run
alone would not have.
### 2026-09-07 — A security job is optional until the stable aggregate requires it

The first OCaml capability-gate draft created an independent job but omitted it
from the final `ci-gate` `needs` list and result loop. It also reused a
contract-step selector that covered only the analyzer package, so ordinary
OCaml package changes could skip the all-OCaml scan. A job's existence is not
enforcement: give it a dedicated registry owner covering the complete protected
path, expose that verdict from detection, and include its result in the stable
required aggregate. Contract-test all three links.

The same audit caught a copied POSIX coverage command in `BUILD_windows`.
Every Windows front is executed one line at a time by `cmd /C`; use the
established `set VAR=value&& command` and `for %f in (...) do ...` forms, then
verify the literal Windows front rather than assuming a same-named file is a
portable translation.

### 2026-09-08 — Prove the JIT entry path explicitly

A compiled-callback regression used execute_with_jit and hit its deliberate
VM entry execution. Eager compilation there installs handlers for subsequent
calls; it does not directly execute the entry binary. For a compiled-entry
proof use compile(), assert is_compiled(), then execute(), with a callback
counter and an interpreter callback that fails if selected.

### 2026-09-08 — Keep refusal probes independent of artifact Debug

An encoded CIL refusal test tried formatting the entire Result, but its success
artifact does not implement Debug. Match Err and Ok separately so diagnostics
only require the error's Display implementation. Run Cargo from the Rust
workspace (or supply a manifest); the repository root has no Cargo.toml.

### 2026-09-08 — Keep inspection paths relative to the actual working directory

A source inspection repeated the repository-relative prefix while already in
the Rust workspace, producing a doubled path. Use workspace-relative paths
there. Python's Windows console also rejected non-CP1252 source characters;
use PowerShell Get-Content or explicitly UTF-8 output when displaying source.

### 2026-09-08 — BEAM output proof must preserve stdout and integer width

Oct's first observable BEAM probe exposed three separate gaps: print_i64 was
rejected, the matrix runner discarded all stdout, and arbitrary-precision
arithmetic returned -1/300 instead of u8 255/44. Fixing only builtin acceptance
would hide both later defects. Compare real output as well as a return marker,
and mask at operation boundaries rather than only when printing.

### 2026-09-11 — Regex anchor semantics do not transfer between engines

A validator built for one backend was copied to another with its `$` intact.
Rust's `regex` treats `$` as end-of-haystack and refuses a trailing newline,
which is what an injection analysis of `host_effect_symbol_re` rested on. PCRE2
(Qt's `QRegularExpression`) and ICU (Swift's `NSRegularExpression`) both concede
a subject-final terminator through `$`, so `^\.?[A-Za-z0-9_-]{1,16}$` accepted
`"apkg\n"` in the shipped Qt handler. ICU concedes `\r\n`, `\r`, U+2028, U+2029
and NEL as well; PCRE2's width is fixed by its build-time newline convention,
so `ANYCRLF`/`ANY` builds concede more than an `LF` build does — measuring one
machine does not establish the language's behaviour.

Use `\A` and `\z`, which are absolute in both. Not `\Z`: in PCRE2 it makes the
same concession `$` does, so it renames the hole rather than closing it.

The failure this allowed was silent, which is what made it survive review: a
glob of `*apkg\n` is rejected nowhere downstream, it simply matches no file, so
the dialog opens empty and reports nothing. Having answered the anchor question
once, for a different engine, is exactly what stopped it being re-asked. Ask it
per engine, and prove the answer by running that engine.

### 2026-09-11 — Read each host's threading contract; do not mirror the first one

Engram's Qt effect handler answers file-dialog effects inline under
`Qt::DirectConnection`, which is correct there because the settle runs on the
event-loop thread. Mirroring that shape into the SwiftUI handler would have been
wrong twice: `settleEffects` is not guaranteed to run on the main thread and
`runModal()` off-main is invalid, and the host's lock is held across the handler
call, so a modal dialog would block `applyProps()` — which SwiftUI calls every
frame. The documented escape, `DispatchQueue.main.sync`, is the exact wedge the
host warns against.

Qt is the outlier, not the template. SwiftUI, Compose, Flutter and XAML all hold
a lock or monitor across the handler and all expose `deferEffect` for precisely
this case; each host's own documentation says so. A 4-to-1 split would have been
gotten backwards three more times by copying whichever backend shipped first.

Deferring inverts the risk and the code must be shaped for it: a deferred effect
leaves the fail sweep, so a path that forgets to answer no longer degrades to
"failed" — it wedges the app permanently, because the runtime gates snapshot and
restore on nothing being pending. Have the dialog function RETURN an outcome so
every path funnels to exactly one completion call.

### 2026-09-11 — A migration's blast radius is every assertion written against the old shape

Moving Engram's SwiftUI backend off its `engram-capi` override touched far more
than the emitter, the manifest and the package tests. Three further places
encoded the architecture being replaced, and each failed in a different way:

- `scripts/build-native.sh` wired a `CEngram` static library and asserted that
  the built binary contained defined `_eg_` symbols. After the migration a
  CORRECT build has zero, so the check failed exactly the configuration it
  existed to protect. CI caught this; reasoning had not.
- `engram_release.py` had an INDEPENDENT copy of the same symbol gate, so the
  publish step would have failed even with the build script fixed. Fixing one
  copy of a duplicated check is not fixing the check.
- Ten release tests built fixtures in the old shape. One of them asserted that
  an executable without engine symbols must be refused — i.e. it required the
  broken architecture and rejected every correct build.

The worst consequence was in none of those: the `.app` bundling copied only the
executable, which was sufficient while the engine was statically linked INTO it.
With the engine now a resource, the shipped app contained no engine at all, and
no assertion noticed, because the old check looked inside the binary where the
engine used to live.

Before migrating a backend, enumerate what asserts its current shape: the
emitter, the manifest, the package tests, the CI lane, the release script, the
release script's tests, and whatever validates the shipped artifact. Each one
encodes an assumption the migration invalidates.

### 2026-09-11 — Verifying a bundled-resource app in place cannot fail

The SwiftUI `.app` was checked by launching it and confirming it stayed alive.
It did, and the verification was worthless: SwiftPM's generated
`resource_bundle_accessor.swift` looks for the resource bundle at
`Bundle.main.bundleURL/App_App.bundle` and then falls back to an ABSOLUTE
build-machine path baked in at compile time. Running the app where it was built
resolves that fallback, so a bundle placed somewhere the accessor never looks
still runs — on that machine, and nowhere else.

The resource bundle belongs at the `.app` ROOT, not in `Contents/Resources`.
The wrong placement fatal-errors with `could not load resource bundle` and exit
133 on every other machine.

Test it by copying the artifact away from its build tree and deleting the build
directory first. A launch test that cannot fail is worse than none, because it
reports success.

The assertion guarding this missed it for the same reason: it used
`find -path '*/Runtime/libmosaic_app.dylib'`, which matches at any depth and so
cannot distinguish the layout that runs from the one that crashes. Assert the
exact path when the layout is what matters.

### 2026-09-12 — `path` is a special zsh parameter

In zsh, the lowercase array parameter `path` is tied to the uppercase `PATH`
environment variable. A shell loop written as `for path in ...` therefore
replaces the executable search path on its first iteration, after which even
`git` and `go` appear to be missing.

Use a task-specific variable such as `sparse_dependency` for path lists. This
also follows the general rule against repurposing common shell and system
option names.

### 2026-09-13 — a separator-joined key is safe if the RAW parts are constant

`sql-vm`'s `apply_distinct` builds a row key as `format!("{col}={val:?}")`
joined by `,`, with the column name interpolated raw. That is the composite-key
separator-injection shape this repo has fixed twice — card ids in
`engram-core-wasm`, the provenance merge key in `engram-core` — and it was
carried as a suspected third instance for weeks.

It is not that bug, and the reason is worth having because it decides the whole
class:

- the column names are raw, **but they are also fixed across every row of one
  DISTINCT**. `apply_distinct` only compares rows within a single result set, so
  a hostile alias contributes the same constant prefix to every key and cannot
  shift one row's boundary relative to another's;
- the values vary, but `SqlValue`'s derived `Debug` quotes and escapes `Text`
  and brackets `Blob`, so the rendering is injective.

A constant prefix plus an injective rendering is injective. Measured before
concluding: 864 rows over hostile aliases (`x=Int(1),y`, `a,b`, duplicate and
empty names) and values whose rendered form carries `,`, `=` and quotes — zero
collisions. The two keys that DO match come from queries with different column
counts, which are never compared to each other.

**The test to write is the one that keeps it true.** The load-bearing half is a
`derive`, one `impl` away from a hand-written `Debug` that prints text raw — so
the gate pins the escaping, not the absence of a collision that cannot happen.
Mutation-checked with exactly that `impl`.

So: before filing a separator-joined key as injectable, check BOTH halves —
whether the varying parts are injectively rendered, and whether the raw parts
are constant across everything the key is compared against. The shape alone does
not decide it.

And the corollary about reviews: a reviewer's "this is the same shape, here is
an exploit" is a lead, not a finding. This one came with a concrete-looking
example (`SELECT a AS "x=Int(1),y", b`) that does not actually collide — the
alias is constant within the query, so it cancels. Reproduce the exploit before
fixing it; implementing a fix for a bug that does not exist costs the same
review budget as a real one and adds code nobody can justify later.

And the same discipline applied to your OWN measurement, which is the harder
half, because a number feels like evidence.

Scoping style-drop reporting for the Flutter backend (#12022), I extracted the
handled keys from `style_prop_to_container_arg` — a single `match` over property
names whose own doc says "unknown props produce `None` and are silently dropped"
— diffed them against every property authored in the repo's `.msl` files, and
got a confident answer: **59 properties dropped**.

The number was wrong, because the premise under it was. That function is one
lowering path, not the lowering: `font-size`, `text-align` and `border-radius`
are handled by other functions entirely (`props.get("border-radius")`,
`base.get("text-align")`, `.get("font-size")`). Flutter has 48 scattered
lookups; Qt has 81. Neither has the single match the three reporting backends
share, which is exactly why they are the two that do not report.

A reporter built on that measurement would have emitted roughly 53 false drops —
the SwiftUI `gap` bug fixed the same day, at ten times the scale, and it would
have looked authoritative because it came with a count.

The check that would have caught it costs one grep: before concluding that a
function is the whole of something, grep for a property it does NOT handle and
see whether the codebase handles it elsewhere. A measurement inherits every
assumption in the thing being measured, and states none of them.
