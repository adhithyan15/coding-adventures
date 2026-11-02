## Unreleased — heap parameters under GC pressure (#18492)

`lang_matrix` gains `twig_heap_parameter_survives_gc_pressure_in_its_callee`:
three Twig programs pass a fresh pair to a function (directly, as another
function's return value, and as a lambda's captured variable) whose body first
allocates 4 MiB of lists — four times the native collector's threshold — and
then reads `(car p)`. Every program must exit 7 on NativeAot, LLVM, the VM and
the JIT (plus WASM and the JVM for the two non-closure shapes; both columns fail
on a lambda that captures a pair even without allocation, unrelated to GC). The
native fix itself, and the compacting-collection reproduction that failed before
it, live in `twig-aot`.
