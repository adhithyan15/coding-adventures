---
category: Testing & coverage
---

# A runner that cannot import by name must verify the compiled export by symbol and source file, or its source digest describes nothing

The Python and TypeScript Warrant lanes load the library named in `evidence.bindings`
(`importlib`, `import()`), so "the code under test" and "the code the manifest
digests" are tied by the loader: the module's real path must lie under the declared
source root. A Go program has no such loader; the exports are fixed when the entry
program is compiled. A Go lane that merely trusted the registry the entry program
handed it would digest a source tree while calling anything.

The Go lane therefore never calls the library function it is given as `Impl`. It
asks the runtime: `runtime.FuncForPC` must name exactly `<import>.<export>` (a
closure, a method or another package's function has a different symbol) and the
function's recorded file must be an absolute path under the `source` root after
symlink resolution. A build with `-trimpath` records no usable path, so the check
fails closed instead of guessing.

Do differently: when a language cannot load code by name, do not drop the
"the digest describes the code that ran" guard because the loader is gone. Rebuild
it from whatever identity the runtime exposes (symbol name, defining file), and
test it with a closure, a wrongly named function and a source root elsewhere.
