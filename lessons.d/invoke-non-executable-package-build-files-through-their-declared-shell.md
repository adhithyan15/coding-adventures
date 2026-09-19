---
category: BUILD files & dependency management
---

# Invoke non-executable package BUILD files through their declared shell

Some repository `BUILD` files are scripts without an executable mode bit.
Running `./BUILD` then fails before the gate starts. Inspect or follow the
package convention and invoke these files as `bash BUILD` from the package
directory.
