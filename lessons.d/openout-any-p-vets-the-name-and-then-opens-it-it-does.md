# `openout_any=p` vets the NAME and then opens it — it does not resolve symlinks, so guarding one output filename guards one of eight

Round two of the same security review. The `book.pdf` symlink guard added in round
one — itself a review finding — was **still wrong**, in the same shape as the bug it
fixed: it enumerated a filename instead of banning a category.

A XeLaTeX run writes at least eight files into the book directory:

    book.aux  book.log  book.toc  book.out  book.xdv  book.pdf
    book.fdb_latexmk   book.fls        <- latexmk's own, written from Perl

`openout_any=p` looks like it covers these and does not. It is a **name** check: it
rejects absolute paths, `..`, and dotfiles, and then hands the name to `fopen(name, "w")`,
which follows the link. `book.fdb_latexmk` and `book.fls` never reach a TeX-side check at
all, because latexmk writes them itself with a plain Perl `open(…, ">")`.

So `<track>/book/book.aux -> /home/runner/.ssh/authorized_keys` is an **arbitrary write as
the build user, from a pull request, with no shell escape and no `latexmkrc` involved.**
Seven doors stood open next to the one that got locked.

**The rule: ban the category, do not enumerate the cases.** The fix moved into the
tree-walking lint, which now refuses *any* symlink under the book tree. Cost of the
blanket ban: zero — `git ls-files -s <tree> | awk '$1=="120000"'` returned nothing, and no
curriculum book has a reason to contain a link. Cost of the enumeration: one missed
filename is a full compromise, and the list grows whenever XeLaTeX or latexmk decides to
write something new. An allowlist of safe things beats a denylist of dangerous ones, and
"the file XeLaTeX happens to write today" is a denylist.

**`.gitignore` is not a boundary.** `book.aux` and friends are ignored — and `git add -f`
commits them anyway. Ignoring a path says where files come from, not what may exist.

**Corollary on where the guard belongs.** It went in the Python lint that already walks
the tree, not in the shell script, because the walk is one pass that already runs twice
and the shell script would need the check at every write site. Put a categorical ban where
the enumeration already happens.
