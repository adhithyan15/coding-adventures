### Security — the books workflow ran `latexmk` unhardened, and its compile gate was never wired up

- **Fixed arbitrary code execution in `.github/workflows/human-languages-books.yml`.**
  The job compiled every book with `latexmk -xelatex … book.tex` after `cd`-ing
  into `code/learning/human-languages/<track>/book`. `latexmk` reads `latexmkrc`
  / `.latexmkrc` from its working directory and hands them to Perl's `eval`, and
  that directory is repository content — so a pull request adding
  `<track>/book/latexmkrc` executed arbitrary Perl on the runner before any TeX
  was parsed. The same invocation ran with TeX Live's *restricted* shell escape
  rather than none. Demonstrated by execution, not by inspection: a benign
  marker `latexmkrc` was written by the old command line and not by the new one.
- **Root cause was drift, so the fix removes the second call site rather than
  copying flags into it.** `-norc -r code/scripts/latexmk-safe.rc` was already
  correct in `check-book-compile.sh`, `build-books-locally.sh`,
  `verify-human-languages.sh` and every track's `book/build.sh` — and absent from
  the workflow, the only place the books are compiled at scale. The workflow now
  invokes `check-book-compile.sh` itself, so there is one hardened invocation in
  the repository and CI cannot drift from it again.
- **Added `code/scripts/check_no_book_latexmkrc.py`**, a repository lint that
  fails if any casing of `latexmkrc` / `.latexmkrc` appears under the book tree,
  plus 15 unit tests. Defence in depth: a flag protects the call sites somebody
  remembered to type it at, and this protects the ones not written yet. It
  reports an unreadable directory as `COULD NOT DETERMINE` with its `errno`
  named, and exits non-zero — never as "clean".
- **Added a CI step that verifies the hardening was in effect** by grepping every
  `book.log` for XeTeX's own `write18 enabled` banner, rather than trusting that
  the flags were typed. This catches a moved rc file and the older-`latexmk`
  quirk where `-xelatex` overwrites `$pdflatex` and the rc's `$xelatex` is never
  consulted.
- **`check-book-compile.sh` gained `--strict`, and CI now runs it.** The script
  was the only gate proving the generated LaTeX actually compiles and no workflow
  referenced it; worse, with no SVG-to-PDF converter installed it printed
  `compiled 0, skipped 1, failed 0` and exited **0** — a gate reporting success
  having verified nothing. `--strict` turns every "could not verify" into a
  failure that names the missing dependency, and fails a run that compiled zero
  books. Local runs stay lenient and now say so in their own output, so a local
  pass is never mistaken for a CI-grade pass.
- **`check-book-compile.sh` now pins `openout_any=p`** rather than inheriting the
  local distribution's `texmf.cnf` default. `TEXMFOUTPUT` is deliberately left
  unset and the reason is recorded in the script: files under it are *exempt*
  from the paranoid check, so setting it would widen what TeX may touch.
- Changes to `check-book-compile.sh`, `latexmk-safe.rc` and the new lint now
  appear in the workflow's path filters. They did not before, so a pull request
  editing the script that compiles every book never ran it.
- **Split the write-scoped token away from the job that executes pull-request
  code.** The old `build-and-publish` job held `permissions: contents: write`
  on a `pull_request` trigger while running `npm ci`, a TypeScript build, seven
  `node` checks and XeLaTeX over repository content — and `actions/checkout`'s
  default `persist-credentials: true` left that token in `.git/config` inside
  the workspace latexmk enters. Hardening the latexmk call while leaving that in
  place would have fixed one door in a room with no walls. Now `build` has
  `contents: read` and `persist-credentials: false`; a separate `publish` job
  holds `contents: write`, runs no repository code, and is gated at job level on
  a push to `main`.
- **The compile now emits a `--manifest` of what it actually built, and the
  collection step publishes from that.** Re-deriving the list with
  `find -type d -name book` asked a different question: `check-book-compile.sh`
  skips a directory with no `book.tex`, so a pull request adding nothing but
  `<track>/book/book.pdf` would have had an attacker-authored file uploaded as a
  build artifact and, on `main`, published to Pages and attached to the Release.
  Verified by planting exactly that file and watching it stay out of the
  manifest.
- Added a `book.pdf` symlink guard to `check-book-compile.sh` (the adjacent
  figure-PDF guard existed; this one did not) and a matching check on the
  consuming side, so the two steps need not trust each other.
- Deleted the workflow's own SVG-to-PDF conversion step. It duplicated what
  `check-book-compile.sh` does per track, and unlike the script it wrote to an
  unchecked derived path, so a committed `figures/diagram.pdf` symlink made
  `rsvg-convert` open an arbitrary runner-writable path for writing.
- The shell-escape verification step now counts the logs it read and fails if it
  read fewer than the compile reported. A loop over zero files left `offenders`
  at 0 and printed a confident all-clear — the same hollow-gate shape as the
  `--strict` fix, in the step meant to prove the fix works.
- The latexmkrc lint runs twice: early for a fast failure, and again immediately
  before the compile. Between the two, the job runs `npm ci` and seven `node`
  scripts, all pull-request-controlled and all able to write a `latexmkrc` after
  the early check has passed.
- `npm ci` for `human-language-data` now passes `--ignore-scripts`, matching its
  four sibling installs. It was the only one without it, so a dependency's
  `postinstall` ran on a pull-request runner.
- **Banned symlinks outright under the book tree, replacing a filename-based
  guard that covered one of eight cases.** A XeLaTeX run writes `book.aux`,
  `book.log`, `book.toc`, `book.out`, `book.xdv`, `book.pdf` and latexmk's own
  `book.fdb_latexmk` and `book.fls` into the book directory. `openout_any=p`
  vets the *name* and then opens it, so it follows a symlink for every one of
  them — and the last two never see a TeX-side check at all, being written from
  Perl. `<track>/book/book.aux -> ~/.ssh/authorized_keys` was therefore an
  arbitrary write as the build user, from a pull request, needing no shell
  escape. Guarding `book.pdf` alone locked one door of eight. Enumerating the
  dangerous cases loses to banning the category; there are zero symlinks under
  the tree today, so the ban costs nothing. `.gitignore` is not a boundary here
  — those names are ignored, but `git add -f` commits them anyway.
- The lint is renamed `check_book_tree_hygiene.py` to match what it now
  enforces, and reports a symlink with its target rather than only its path.
- **The shell-escape verification step now iterates the manifest** rather than
  re-finding logs with `find -path '*/book/book.log'` and comparing counts.
  `-path` matches at any depth, so a committed decoy log could pad the count and
  mask a real one that had dropped out; a count was standing in for an identity
  check. Iterating the manifest removes the count comparison entirely.
- A failed manifest append is now fatal. The script runs `set -uo pipefail`
  without `-e`, so a silent failure would have left a book out of the manifest
  while the summary still read "every selected track was compiled and verified"
  — the vacuous-pass class this work exists to remove. `--manifest=` with an
  empty value is now an error rather than a silent no-op.
- Added `code/scripts/tests/test-check-book-compile-guards.sh`, which runs the
  real script against a real symlink and a real planted `book.pdf`. It skips the
  symlink case where the filesystem cannot create one and runs it on CI.
  Confirmed non-vacuous by mutation: removing the `book.tex` guard turns it red.
- **`check-book-compile.sh` now sweeps its own book directory for symlinks**
  before writing anything, rather than relying on a lint only CI invokes. The
  script is documented as the one a human runs locally, and locally nothing runs
  that lint — so `git checkout <branch> && ./code/scripts/check-book-compile.sh`
  on Linux or macOS still wrote through `<track>/book/book.aux -> ~/.ssh/…`,
  which is author-controlled content, not merely a destructive overwrite. The
  PR's own thesis one level up: a control protects only the call sites that
  invoke it.
- **A skipped symlink test is now a failure where the capability must exist.**
  Both symlink suites skip on a filesystem that cannot create links, and nothing
  asserted the probe had succeeded on the runner — so a broken runner image
  would silently stop exercising the security tests while the step stayed green.
  CI sets `REQUIRE_SYMLINK_TESTS=1` and the skip becomes fatal. Verified in both
  directions.
- **A symlink named `latexmkrc` is now reported under both bans, not just one.**
  The symlink advisory ends "Replace the link with the real file, or delete it",
  which for that path instructs the author to create a real `latexmkrc` — the
  artefact the other ban exists to keep out. De-duplicate the failure, never the
  guidance.
- `--book-root=` with an empty value is rejected rather than silently reverting
  to the real corpus (the same bug as `--manifest=`, one line apart), and the
  seam is gated behind `CHECK_BOOK_COMPILE_SELF_TEST=1`.
- The shell-escape verification step checks `-L` before `-f` on each `book.log`,
  since `-f` follows a link and would read an attacker-chosen file.
- **Fixed `check-book-compile.sh` and `verify-human-languages.sh` shipping mode
  `100644` while documenting `./code/scripts/<script>.sh` as their usage.** The
  documented invocation had never worked on a fresh Linux or macOS clone —
  `Permission denied`, exit 126. Hidden by two things at once: Windows does not
  model the executable bit, so the authoring platform could not detect it, and
  every automated caller used `bash <script>`, which works either way. So the
  only broken invocation form was the one only humans use, and no gate used it.
  Found by the new guards suite, which runs the script the way the docs say to.
  Fixed by mode rather than by changing the caller: `bash "$SCRIPT"` would have
  gone green while leaving the documented command broken. The suite now asserts
  the **git index** mode, since a filesystem `-x` test is vacuous on Windows.
- Read-side TeX exposure (`openin_any`) and third-party action SHA pinning are
  tracked separately rather than guessed at here — see the linked issues. An
  unverified control is worse than an acknowledged gap.

