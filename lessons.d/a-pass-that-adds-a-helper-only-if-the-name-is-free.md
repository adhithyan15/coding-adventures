---
category: Compiler / VM / language pipeline
---

# A pass that adds a helper only if the name is free hands that name to whoever took it first

`iir-builtin-lowering`'s `list_ops` and `closure_heap` add a helper function
and then rewrite calls to point at it by name. They guarded the add with
`if !module.functions.iter().any(|f| f.name == HELPER)`, which was written
for idempotence. It also meant a program's own function with that name was
kept, and every `length` or closure call went to it.

The same shape showed up twice more in one security review. `twig-aot`'s link
step patched any call whose symbol named a module function, so a program
function called `__twig_print_i64` received `print`'s runtime call. And
`twig-ir-compiler` lifted lambdas into `__lambda_N`, which Twig source could
also define, giving the module two functions of one name.

**Fix:** a name the toolchain generates or links against is *claimed*, not
borrowed. Before rewriting, check that a function under the helper's name is
missing (add it) or identical to the helper (it ran before). Anything else is
an error that names the pass (`helper_names::HelperNameError`). At the link
step, refuse a module function named like a runtime symbol, and refuse
duplicate function names. At the source, reserve the prefixes.

**Do differently:** whenever code looks a function up by a name it chose
itself, ask what happens if that name already exists. "Skip if present" is
only idempotent when "present" is proven to mean "mine".
