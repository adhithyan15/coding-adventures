# Hardening added call-site-by-call-site reaches every call site except the one that matters — and the "belt and braces" flag can be the only one working

`latexmk` reads `latexmkrc` / `.latexmkrc` from its **working directory** and hands
the contents to Perl's `eval`. Every book in this repo is compiled with that
directory set to `code/learning/human-languages/<track>/book`, which is pull-request
content. The control is `-norc`.

`-norc -r code/scripts/latexmk-safe.rc` was present and correct in
`check-book-compile.sh`, `build-books-locally.sh`, `verify-human-languages.sh`, and in
all 23 tracks' `book/build.sh` **and** `book/build.ps1`. It was absent from
`.github/workflows/human-languages-books.yml`, which is the only place the books are
compiled at scale, on every pull request, in a job holding a `contents: write` token.

Nothing detected that, because a grep for the *protection* over the scripts directory
returns plenty of hits and looks reassuring. The grep that finds this is the one over
the **invocations**:

    grep -rn 'latexmk\|xelatex\|pdflatex' .github/workflows/

and then checking each hit for the flag — not grepping for the flag and counting.

**The general rule.** When you harden a dangerous call, the unit of work is *every
invocation in the repository*, enumerated from the dangerous verb. Fixing them one at a
time as you encounter them guarantees the set you never encountered stays unfixed, and
the ones you never encounter are disproportionately in CI — nobody runs CI locally, so
its call sites are not the ones you trip over. Better still, delete the second call site:
this fix made the workflow invoke `check-book-compile.sh` rather than maintain a
parallel `latexmk` command line, so there is now one invocation and drift is not
possible.

**A flag is only present where someone typed it; make the payload impossible instead.**
`code/scripts/check_no_book_latexmkrc.py` now fails the build if any casing of
`latexmkrc` appears under the book tree. That protects call sites not yet written.

**Assert the control at runtime, not in review.** XeTeX writes its shell-escape state
into every `book.log`: `" restricted \write18 enabled."`, `"\write18 enabled."`, or —
when off — no line at all. CI now greps the logs the build just produced. This is the
only check that catches the failure mode below, which no amount of reading the YAML
would find.

**The measured control matrix (latexmk 4.88), which is not the one you would guess:**

    invocation                                    latexmkrc    shell escape
    --------------------------------------------  -----------  -------------------
    latexmk -xelatex ...            (old CI)      EXECUTED     restricted, enabled
    latexmk -norc -xelatex ...                    not read     restricted, enabled
    latexmk -norc -r latexmk-safe.rc -xelatex     not read     DISABLED
    shell_escape=f latexmk -xelatex ...           EXECUTED     restricted, enabled

Two things fall out. `-norc` alone does **not** turn shell escape off — it only stops
the rc being read; the `-r` that loads `$xelatex = "xelatex -no-shell-escape %O %S"` is
what disables it, so the two flags are not redundant with each other. And
`shell_escape=f` in the environment did nothing on this box: **MiKTeX ignores the
kpathsea `shell_escape` and `openout_any` environment variables entirely** (`openout_any`
set to `p`, `a`, or unset all behaved identically, all blocking `../` writes under
MiKTeX's own configuration). TeX Live honours them; MiKTeX does not. So a Windows
verification of an env-var-based TeX hardening proves nothing either way — say so
rather than reporting it as tested.

**`TEXMFOUTPUT` is not a sibling of `openout_any`.** It reads like another lockdown knob
and is the opposite: paths under `TEXMFOUTPUT` are *exempt* from `openout_any=p`. Setting
it widens what TeX may write. Leave it unset.
