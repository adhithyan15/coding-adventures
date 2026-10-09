---
category: TypeScript / JavaScript
---

# An npm advisory with no patched release survives every audit fix; only removing the path that pulls it in clears it

**Context:** the 2026-10-09 Dependabot sweep. `npm audit fix --package-lock-only`
under npm 11 cleared 476 of 486 vulnerable lockfiles in one pass, and the
other ten would not move.

**What went wrong:** two of the remaining advisories had no fixed version to
move to:

    $ npm view braces dist-tags.latest       # GHSA-vfj7-8cjw-p6xm: <=3.0.3
    3.0.3
    $ npm view sprintf-js dist-tags.latest   # GHSA-hp3w-g68c-fv3c: <=1.1.3
    1.1.3

`npm audit` still printed `fix available via npm audit fix --force` for both.
That "fix" is a *different parent version* that happens not to pull the
package in, and npm's choice there can be absurd. For `sprintf-js` it offered
`electron-builder@26.5.0`, a downgrade from the locked 26.17.0, because that
version's graph happened to dodge `global-agent`. Running `--force` would have
traded one moderate advisory for every advisory 26.5.0 still carries.

**What to do instead:**

1. Check `npm view <pkg> dist-tags.latest` against the advisory's range before
   chasing a fix. If the latest release is inside the range, no bump exists.
2. Find the path with `npm explain <pkg>` and decide which *direct*
   dependency to change. For `braces`, jest 29 → 30 removed
   `micromatch`/`braces` entirely, which was a deliberate major bump with the
   tests run, not `--force`.
3. If no reasonable parent change exists (e.g. `sprintf-js` through
   `argparse` 1 and through `roarr`), record it in
   `code/specs/EXTERNAL-DEPENDENCY-ALERT-BACKLOG.md` as "open, no upstream
   fix" with the removal that would clear it. Don't leave it as an unexplained
   red row in the audit.

**Related trap, same sweep:** Engram's two lockfiles are *generated*. Their
`package.json` is emitted by `mosaic-package-artifact-builder` and
`mosaic-emit-react`, and `scripts/build-*.sh --update-lock` regenerates them
under a seven-day release-age floor. `npm audit fix` in `npm/electron/` would
have produced a lock that no longer matches the emitter, which
`tests/npm_lockfiles.rs` rejects. Change the pins in the emitter and
regenerate. If a fix is younger than the floor, wait for it and log it as
"open, cooldown" rather than bypassing the floor.
