# WEB14 Go Conduit — cgo + conduit-capi gotchas

**conduit.Application.GetSetting is invalid after Bind().** `conduit_server_bind`
moves (and frees) the native `ConduitApp*` into the server handle on BOTH
success and failure. Any Go closure registered as a handler, before-filter, or
after-hook that calls `app.GetSetting()` at request time will read from a freed
pointer. Fix: pre-capture settings into local strings BEFORE registering the
hooks. Surfaced in conduit-hello's after-hook stamping `x-served-by`.

**cgo Rule 6 / uintptr-as-void*: use cgo.Handle, not raw casts.** Passing a
`uintptr` value of a Go pointer directly to C as `void*` triggers `go vet` Rule
6 warnings (the GC may move the pointer between the cast and the store). The
correct pattern: `cgo.NewHandle(fn)` returns a GC-safe integer handle; cast
`C.uintptr_t(handle)` to `void*` in a C shim (where the cast is invisible to
`go vet`). The trampoline recovers the value with `cgo.Handle(uintptr(ctx)).Value()`.

**Static link the Rust .a by full path, not -lconduit_capi.** On Linux `ld`
resolves `-lconduit_capi` to the sibling `.so` cdylib, not the `.a` staticlib.
Resulting binary fails at runtime (`libconduit_capi.so.0: not found`). Use the
full path in `#cgo LDFLAGS: ${SRCDIR}/.../libconduit_capi.a` instead.

**cgo LDFLAGS native deps differ by OS.** macOS needs `-liconv`; Linux needs
`-lpthread -ldl -lm -lrt -lutil`. The full list comes from
`cargo rustc --release --crate-type staticlib -- --print native-static-libs`.

**Go BUILD files that link a Rust static lib must be self-sufficient (build
the Rust lib themselves).** The `# build-tool: deps=rust/foo` directive ensures
ordering in the *normal* build workflow (which uses the build-tool directly),
but the CodeQL workflow generates a `build-plan.json` of 300+ packages and
processes them in plan order — which may reach `go/conduit` before
`rust/conduit-capi`. Bare `go test` then fails: `libconduit_capi.a: No such
file or directory`. Fix: add a `tools/run-tests.sh` that runs `cargo build
--release` for the Rust crate first (mirroring the C++ pattern), and call
`sh tools/run-tests.sh` from the BUILD file. The deps= hint still fires in
the normal build (making the cargo step a fast no-op); CodeQL gets the Rust
lib on demand. Surfaced in PR #5739.
