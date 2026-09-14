# A script documented as `./script.sh` must ship mode 100755 — and only the platform that cannot detect it will introduce the bug

`check-book-compile.sh` documents its own usage as `./code/scripts/check-book-compile.sh`
and shipped git mode **100644**. The documented command had therefore never worked on a
fresh Linux or macOS clone: `Permission denied`, exit **126**.

Two things hid it, and they reinforce each other:

- **Windows does not model the executable bit.** `[ -x file ]` is true for every readable
  file there, so an authoring box in this repo can neither cause nor detect the problem —
  and the same box is where most of these scripts get written.
- **Every automated caller sidesteps it.** CI ran `bash code/scripts/check-book-compile.sh`,
  which works at 100644. So the one invocation form that was broken was the one only
  humans use, and no gate used it.

`verify-human-languages.sh` had the identical defect. `build-books-locally.sh`,
`generate-compiled-grammars.sh` and `miri-twig-vm.sh` were all already 100755, which is
what marks the other two as oversights rather than a convention.

**Fix the mode, not the caller.** The test that found this invoked `"$SCRIPT"` directly.
The tempting repair is `bash "$SCRIPT"` — the suite goes green in one character. It also
leaves the documented command broken forever, and deletes the only thing in the repo that
was executing it the way a human would. When a test catches a real defect, the defect is
what moves.

**Assert the index mode, not `-x`.** A `[ -x ]` regression test is vacuous on Windows, so
it would pass on the platform most likely to reintroduce the bug. `git ls-files -s` reports
what actually ships and is meaningful everywhere:

    git_mode=$(git ls-files -s -- "$SCRIPT" | awk '{print $1}')
    [ "$git_mode" = "100755" ] || fail

Mutation-check it with `git update-index --chmod=-x` / `--chmod=+x`, which flips the index
mode without touching the filesystem — so it works as a test even on Windows.

**Generalisable check:** for every script whose header documents a `./` invocation,
`git ls-files -s` it. The two are independent facts and nothing in this repo tied them
together.
