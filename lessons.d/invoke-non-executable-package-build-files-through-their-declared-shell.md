---
category: BUILD files & dependency management
---

# Invoke non-executable package BUILD files through their declared shell

Some repository `BUILD` files are scripts without an executable mode bit.
Running `./BUILD` then fails before the gate starts. Inspect or follow the
package convention and invoke these files as `bash BUILD` from the package
directory.

This recurred after correctly switching an X.509 validation command into the
crate directory but then invoking `./BUILD`. Working-directory correctness and
the executable bit are independent: use `bash ./BUILD` for this package.
