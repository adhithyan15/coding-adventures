---
category: Security boundaries
---

# A classifier that tracks cwd as text while the kernel resolves .. physically is redirected by any directory rename, not only by symlinks

**What went wrong.** Kiln's trace classifier (KILN04 D40, tmpfs containment)
skipped its stability rules for paths inside the sandbox's fresh tmpfs, on the
premise that a lookup can only leave the tmpfs through a symbolic link, so "no
symlink made there" meant "every path there is the action's own file or
nothing". The first draft assumed only symlinks could redirect a path. It
ignored that the trace keeps each process's cwd as a TEXT expression which a
directory rename never updates, while the kernel resolves `..` against the real
directory. `chdir $HOME/a/b; rename $HOME/a/b $HOME/b2; mkdir $HOME/a/b;
unlink ../work/repo/f` then resolves lexically to the action's own scratch file
and physically to the bind-mounted checkout: a repository write with no reason
and writes_complete true.

**How it was found.** The security review wrote a working proof of concept
(a test with the exact sequence above, run against the branch). A second hole
of the same family came from the advisor: renaming an ancestor of the checkout
(`mv ~/work ~/y`) under Home, which an earlier rule allowed as a mere "look".
The differential fuzzer had passed 2.8M execs because its model could not
generate either operation (it forbade renaming a directory under a cwd and
could never name an ancestor exactly).

**The fix.** `classifier.tmpMoved`: a successful rename or exchange under the
tmpfs whose source is a directory, or an object the action did not create,
disables containment for the whole action (renaming a regular file the action
created, as the Go build cache and gcc do, stays contained). Rule (a) for
ancestor nodes now covers looks only, never successful write-class calls.

**Do differently.** Before relaxing a path-resolution rule on the grounds that
"nothing can redirect a path", list EVERY operation that changes the
path-to-object binding: symlink creation, rename of a directory (and exchange),
and mount. Check each against every piece of state the model keeps as text or
as a cached object. Then teach the fuzzer model to generate each one (and to
allow what it used to forbid), with a burst that drives the pattern, and prove
it has teeth by disabling the fix and watching the fuzzer fail.
