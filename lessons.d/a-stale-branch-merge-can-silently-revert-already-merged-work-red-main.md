# A stale-branch merge can silently REVERT already-merged work → red main from two green PRs

**Symptom:** `main` stopped compiling. `spreadsheet-core-wasm` 0.15 (SSIO PR4
#7701) called `spreadsheet_io::load_json`, but `spreadsheet-io` on `main` was
back at 0.3.0 with the whole JSON codec gone. `cargo build -p spreadsheet-wasm
--target wasm32-unknown-unknown --release` failed with `cannot find function
load_json in crate spreadsheet_io`.

**Root cause:** PR #7682 ("feat(sir): sir-hashmethods-go … **rebased onto
main**") was rebased on a **stale, pre-#7693 main**. Its merge silently reverted
the ENTIRE #7693 changeset (13 files across spreadsheet-io + json-parser +
json-lexer) as collateral — #7682 was the only post-#7693 commit touching any of
them, and it dragged them back to their old state. #7682's CI was green (its
snapshot compiled); #7701's CI was green (its base still had `load_json`).
Neither CI saw the other, so **two individually-green PRs merged into a red
main**. This also re-opened a CRITICAL DoS (the json-parser recursion depth cap
went with the revert).

**How to detect fast:** when a symbol "vanishes" that you know was merged,
`git log --oneline <good-merge>..origin/main -- <file>` shows the culprit, and
`git show origin/main:<file> | grep <symbol>` confirms it's gone. A reverted
crate's **version number rolls backward** (0.4.0 → 0.3.0) — a dead giveaway.

**Fix:** restore only the affected files to their last-good commit —
`git checkout <good-sha> -- <the exact file list>` — never a broad revert (that
would clobber the stale PR's *legitimate* work). Verify the previously-failing
build command actually passes now. The restore is byte-identical to
already-reviewed code, so no fresh security review is needed.

**Prevention / takeaways:**
- **"Rebased onto main" in a PR title is a smell** — verify its file list is only
  what it claims. A SIR/Go PR touching `spreadsheet-io`/`json-*` is a red flag.
- Green CI on *both* PRs does NOT mean their *combination* is green. A shared
  crate that one PR removes and another PR starts calling is invisible to both
  pre-merge checks. (See also: "Diff branch full file list vs origin/main before
  push; stale worktrees revert files.")
- When you inherit a broken main mid-loop, **fixing main is the priority** — a
  focused hotfix PR unblocks the whole repo, not just your feature.

Discovered: 2026-07-06, SSIO PR5 setup (found `main` non-compiling; hotfix #7709).
