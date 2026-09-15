# Removing a custom CRT entry point also requires rechecking its library workaround

The Windows AOT link stopped overriding `/ENTRY:main` but retained `libvcruntime.lib` beside dynamic `vcruntime.lib`. A real Twig `42` execution attempt failed in `lld-link` with duplicate `__vcrt_InitializeCriticalSectionEx`; a unit test asserting the old library list had stayed green. Remove the obsolete static library and validate actual executable startup, output, and heap operations. Record the two existing precise-GC smoke-test early returns separately from executed passes.
