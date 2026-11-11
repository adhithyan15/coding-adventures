---
category: Cross-platform & Windows BUILD_windows
---

# BUILD_windows drift is mechanical, so audit it instead of waiting for a dependent rebuild to fail

Almost no PR runs the Windows shard, so a `BUILD_windows` that drifted from
`BUILD` stays latent until an unrelated PR rebuilds the package as a dependent
and fails on code it never touched. Several rounds of that produced the same
few shapes, all detectable by reading the files, so
`code/scripts/audit_build_windows.py --check` now runs in the "Repo-wide
metadata contracts" job and the shapes below should not recur.

**The build tool's selection rule decides what Windows runs.** Windows picks
`BUILD_windows`, else `BUILD`. It never uses `BUILD_mac_and_linux`. A package
with a plain `BUILD` and no `BUILD_windows` therefore runs that `BUILD` through
`cmd /C`, line by line.

**The classes.**

- vitest left behind: a TypeScript package moved to `node --test` (its
  `package.json` dropped vitest, its `BUILD` runs `npm run build` and
  `npm run test:coverage`) but its `BUILD_windows` still ran `npx vitest`. Fix:
  make `BUILD_windows` the same three lines as `BUILD`. A Starlark `BUILD` with
  `test_runner = "node"` expands to `npm ci`, `npm run build`,
  `npm run test:coverage`.
- `set PERL5LIB=..\x\lib;%PERL5LIB% && prove ...`: CI's MSYS perl splits
  PERL5LIB on `:`, so the entry is never found. Fix: `prove -I..\x\lib -l -v t`
  (or `-I` on each `perl` call). Dropping a `set` that sits beside `-I` flags
  is enough.
- Ruby `standardrb` with no `BUILD_windows`, or with one lacking
  `--except Layout/EndOfLine`: Windows StandardRB wants CRLF and the repo forces
  LF. Fix: a `BUILD_windows` identical to `BUILD` except for that flag.
- Different test runners in the two files, and POSIX-only syntax in a `BUILD`
  that Windows falls back to. The second is only reported for packages not on
  the audit's `KNOWN_POSIX_FALLBACKS` list.

**Not mechanical, so not audited.** Ruby library code that passes a size or
header value to a native method can overflow a 32-bit C `long` on Windows
(found in lz78 and lzss). That needs a person reading the code.

**When you fix a Windows front without a Windows machine,** run the translated
commands on Linux (swap `\` for `/`) and confirm they pass with `PERL5LIB`
unset. That proves the `-I` list is complete.
