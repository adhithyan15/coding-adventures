---
category: Native extensions & FFI
---

# A wrapper must forget a native pointer before the call that consumes it, not just flag it

`conduit_server_bind(host, port, app)` takes ownership of `app`: it moves it
into the server on success and frees it (with every registered closure) on
failure. The Go and C++ `Application` wrappers set a `consumed` flag after the
call, but only the destructor/`Free` read that flag. Every registration
method (`Route`, `Before`, `After`, `NotFound`, `OnError`, `Set`,
`GetSetting`, and a second `Bind`) still passed the stale pointer, so any
call after `Bind` was a use-after-free in Rust.

**Fix:** clear the stored pointer *before* the consuming call (keep a local
copy for the call itself), and route every method through one `live()`
accessor that panics or throws on a consumed wrapper. A flag nobody reads is not
a guard; nulling the field is what makes a forgotten check fail loudly and not
corrupt memory.

**Ordering details:**
- Run the guard before allocating whatever you hand to native code
  (`cgo.NewHandle`, `new Handler`), so a refused call leaks nothing.
- In C++, call it in its own statement: argument evaluation order is
  unspecified, so `f(live(), new Handler(h))` may allocate first.

**Do differently:** for any binding of an API whose function "consumes",
"takes" or "moves" a handle, audit *all* other methods on the wrapper, not
only its destructor, and add a test that each one is refused after the
consuming call, on both the success and the failure path.
