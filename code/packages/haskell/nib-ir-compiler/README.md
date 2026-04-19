# nib-ir-compiler

Haskell `nib-ir-compiler` lowers checked Nib ASTs into the local
`compiler-ir` package. It follows the shared Nib ABI: function parameters live
in `v2+`, return values leave in `v1`, and source functions are labeled
`_fn_NAME`.
