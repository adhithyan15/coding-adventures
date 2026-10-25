---
category: BUILD files & dependency management
---

# BUILD recipes may require the package working directory

A repository package's `BUILD` file can contain paths relative to its own
directory even when the file is invoked through a shell from the repository
root.  Inspect the recipe first, then run it with the package directory as the
working directory; merely prefixing the path to `BUILD` can make valid project
paths look missing.  Also do not assume a recipe is executable when the
repository convention is to pass it to its declared shell.
