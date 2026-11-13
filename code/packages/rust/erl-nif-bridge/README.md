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

`BUILD` / `BUILD_windows` (the same on every OS) run:

1. `cargo test -p erl-nif-bridge`: pins the callback-table order on any platform.
2. `cargo clippy --all-targets -- -D warnings`.
3. `cargo build --release --example nif_smoke`, then
   `escript tests/smoke.escript target/release/examples`. The example is a real NIF
   library built on the crate: it wraps the integer, float, atom, tuple, binary,
   list, map (including the iterator), resource, pid, thread-to-process message and
   error functions. The script compiles `tests/nif_smoke.erl`, loads the library with
   `erlang:load_nif/2` into the real BEAM on `PATH` and checks every result from the
   Erlang side. On Windows this is also the proof that `win_dyn`'s table order matches
   the pinned OTP 27.

A compile check cannot find a bridge bug: a wrong symbol, a wrong constant and a wrong
argument all compile. `enif_make_pid` was declared as an exported function and is a
header macro, so a NIF that called it failed to load (`undefined symbol`); the first
run of the smoke test found it. The recipe needs Erlang/OTP on `PATH`
(`# needs-toolchain: elixir`, which installs the pinned 27.3.4.11). The Windows path is
also exercised by the Elixir NIF packages' `BUILD_windows` (for example
`elixir/gf256_native`).

Not covered by the smoke test: `enif_make_list` and `enif_make_tuple` (variadic, so not
in the Windows table) beyond the one `enif_make_tuple` call on Unix, dirty schedulers,
`enif_select`, and OTP releases other than the one CI installs.
