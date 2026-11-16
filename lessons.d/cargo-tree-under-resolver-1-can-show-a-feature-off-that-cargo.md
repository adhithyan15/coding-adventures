---
category: BUILD files & dependency management
---

# cargo tree under resolver 1 can show a feature off that cargo build compiles on; read the build itself, never the tree

The VOS02 acceptance spike (SH16, the image-build gate) found that `cargo tree
-e features` under resolver 1 can print a feature as off while `cargo build`
compiles the crate with it on: resolver 1 unifies features across the build and
dev dependency graphs and across targets, and the tree view does not always
show that unification. A gate that read the tree would have passed an image that
contained the `ghost` feature.

Do differently: when a property must hold of what was COMPILED (a feature, a
dependency, a flag), read it from the build itself (the recorded build
invocation and its `--cfg feature=` flags, or the compiler's own output), never
from a tree or manifest view of what Cargo would do. Use resolver 2 in the
workspace, and treat any manifest-level feature reading as advisory (VOS02
section 7.2 item 4; the hermetic recorded build is VOS22).
