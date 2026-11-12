# erl-nif-bridge

Zero-dependency Rust declarations of the Erlang/OTP NIF C API (`enif_*`), so a Rust
`cdylib` can be loaded by the BEAM with `:erlang.load_nif/2`. No Rustler, no Erlang
headers at build time.

## How the symbols are resolved

| Platform | Mechanism |
|---|---|
| Linux, macOS | The library leaves `enif_*` undefined; the BEAM's dynamic loader resolves them at `load_nif` time (`-undefined dynamic_lookup` on macOS). |
| Windows | A DLL cannot leave imports undefined and `beam.smp.dll` exports no `enif_*` names. OTP passes `nif_init` a table of function pointers (`TWinDynNifCallbacks`); `src/win_dyn.rs` stores it and exposes same-named functions that call through it. |

`src/win_dyn.rs` is generated: `python3 tools/gen_win_dyn.py` reads the callback-table
order (`tools/otp27_nif_api_order.txt`, from OTP 27's `erl_nif_api_funcs.h`) and the
signatures in the `extern "C"` block of `src/lib.rs`. The table is positional, so the
order file must match the OTP release the Windows CI installs; OTP's header says new
entries are only ever appended.

## Entry point

```rust
erl_nif_bridge::nif_init!("my_module", FUNCS);
```

generates `nif_init()` on Unix and `nif_init(callbacks)` on Windows. A hand-written entry
point must do the same: on Windows call `erl_nif_bridge::win_dyn::win_init(callbacks)`
before returning the `ErlNifEntry`.

## Tests

`cargo test` pins the table order on any platform. The Windows path is exercised by the
Elixir NIF packages' `BUILD_windows` (for example `elixir/gf256_native`).
