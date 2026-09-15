# A lint that only CI runs protects only CI — the script a human runs must carry its own guarantee

Round three of the same review, and the same lesson eating its own tail for the third
time. The general symlink ban was moved into `check_book_tree_hygiene.py` — correct — and
`check-book-compile.sh` never calls it. CI was safe, because the workflow runs the lint
immediately before compiling. But the script is documented as *the same one a human runs
locally*, and locally nothing runs the lint:

    git checkout <contributor-branch>
    ./code/scripts/check-book-compile.sh          # on Linux or macOS

still wrote through `<track>/book/book.aux -> ~/.ssh/authorized_keys`. Not a mere
destructive overwrite, either: `.aux` content is substantially author-controlled through
labels and TOC entries. The two surviving `[ -L ]` guards covered 2 of the 9 files a
XeLaTeX run writes.

This is the PR's own thesis one level up. The thesis was "a flag only protects the call
sites somebody remembered to type it at", and the answer was "move the guarantee into a
lint". Then the lint became the thing only some call sites invoke. **Ask, of every control
you extract into a shared checker: who calls the checker, and what happens to the callers
who do not?**

The fix is four lines in the existing idiom — a whole-directory
`find -type l -print -quit` sweep before any write — after which the narrow `[ -L ]` guards
are genuinely redundant in CI and the script finally carries its guarantee everywhere.
Keeping the narrow guards anyway is right: they cost nothing and they name the specific
file rather than the first link found.
