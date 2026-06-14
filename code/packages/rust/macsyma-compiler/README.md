# coding-adventures-macsyma-compiler

Compiles MACSYMA parser ASTs into `symbolic-ir` trees.

This package depends on the statically linked MACSYMA parser package and keeps
the runtime path free of grammar-file I/O for the future WASM facade.
