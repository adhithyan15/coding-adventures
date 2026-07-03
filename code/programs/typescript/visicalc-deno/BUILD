# visicalc-deno — VisiCalc on Deno Desktop (WASM engine path).
#
# No build step / dependencies to install: Deno resolves the two embedded assets
# from the sibling ../visicalc-html at compile time.
#
#   deno task desktop   # open a dev window (deno desktop -A main.ts)
#   deno task build     # compile a self-contained VisiCalc.app
#   deno task dev       # server only; open http://localhost:8791
#
# FFI variant (main-ffi.ts) — runs the engine as a NATIVE dynamic library via
# Deno.dlopen instead of WASM. This one DOES have a build step: the native
# library must be built + vendored first.
#
#   deno task engine        # build + vendor libspreadsheet_capi (git-ignored)
#   deno task desktop:ffi   # open a dev window (deno desktop -A main-ffi.ts)
#   deno task dev:ffi       # server only; open http://localhost:8792
#   deno task build:ffi     # compile a self-contained VisiCalc-FFI.app
