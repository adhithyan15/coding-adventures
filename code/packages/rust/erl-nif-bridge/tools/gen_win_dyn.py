#!/usr/bin/env python3
"""Generate src/win_dyn.rs: the Windows half of erl-nif-bridge.

WHY THIS EXISTS
===============

On Linux and macOS a NIF library is a shared object that leaves the `enif_*`
symbols undefined; the BEAM's dynamic loader resolves them when
`:erlang.load_nif/2` opens the file.  A Windows DLL cannot do that: the PE format
wants every import to name the module that exports it, and `beam.smp.dll` exports
no `enif_*` names at all (the Windows CI probe confirmed it: the DLL does not even
contain the string `enif_make_atom`).

Instead Erlang/OTP hands each NIF library a table of function pointers.  The
header does this with `TWinDynNifCallbacks`::

    ErlNifEntry* nif_init(TWinDynNifCallbacks* callbacks)
    {
        memcpy(&WinDynNifCallbacks, callbacks, sizeof(TWinDynNifCallbacks));
        ...
    }

and every `enif_foo(...)` in C becomes the macro `WinDynNifCallbacks.enif_foo(...)`.

Rust has no C preprocessor, so this crate does the same thing by hand: it keeps a
copy of the table and offers ordinary functions with the same names and signatures
as the `extern "C"` block in lib.rs, each of which calls through the table.  Code
that uses the crate does not change.

The one thing that must be exactly right is the ORDER of the table, because a
wrong slot silently calls a different function.  The order comes from the real
OTP 27 header and is committed in tools/otp27_nif_api_order.txt.

WHAT THIS SCRIPT DOES
=====================

    otp27_nif_api_order.txt  --->  slot number of every enif_* function
    lib.rs `extern "C" {}`   --->  the Rust signature of every function we use
                  \\                   /
                   v                 v
                     src/win_dyn.rs   (a generated file; do not edit by hand)

Run it from this package directory after changing either input::

    python3 tools/gen_win_dyn.py

Each generated wrapper looks like::

    pub unsafe fn enif_make_atom(env: ErlNifEnv, name: *const c_char) -> ERL_NIF_TERM {
        let f: unsafe extern "C" fn(ErlNifEnv, *const c_char) -> ERL_NIF_TERM =
            core::mem::transmute(slot(idx::ENIF_MAKE_ATOM));
        f(env, name)
    }
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
PKG = HERE.parent
ORDER_FILE = HERE / "otp27_nif_api_order.txt"
LIB_RS = PKG / "src" / "lib.rs"
OUT = PKG / "src" / "win_dyn.rs"

# Functions the Windows wrappers add that lib.rs does not declare in its extern
# block: the 64-bit integer entries only exist in the Windows table (C `long` is
# 32 bits there, so OTP has separate int64 functions) and lib.rs needs them for
# make_i64 / get_i64.  (name, argument list, return type)
EXTRA = [
    ("enif_get_int64", "env: ErlNifEnv, term: ERL_NIF_TERM, ip: *mut i64", "c_int"),
    ("enif_get_uint64", "env: ErlNifEnv, term: ERL_NIF_TERM, ip: *mut u64", "c_int"),
    ("enif_make_int64", "env: ErlNifEnv, i: i64", "ERL_NIF_TERM"),
    ("enif_make_uint64", "env: ErlNifEnv, i: u64", "ERL_NIF_TERM"),
]


def read_order() -> list[str]:
    names = []
    for line in ORDER_FILE.read_text().splitlines():
        line = line.strip()
        if line and not line.startswith("#"):
            names.append(line)
    if len(set(names)) != len(names):
        sys.exit("duplicate name in the order file")
    return names


def extern_block(src: str) -> str:
    """The text of the first top-level `extern "C" { ... }` block that declares
    `pub fn enif_*` functions (the bridge's API declarations)."""
    start = src.index('extern "C" {\n    // -- Integer conversions')
    depth = 0
    i = start
    while True:
        c = src[i]
        if c == "{":
            depth += 1
        elif c == "}":
            depth -= 1
            if depth == 0:
                return src[start : i + 1]
        i += 1


def split_top_level(s: str) -> list[str]:
    """Split on commas that are not nested in (), <> or []."""
    parts, depth, cur = [], 0, ""
    for ch in s:
        if ch in "(<[":
            depth += 1
        elif ch in ")>]":
            depth -= 1
        if ch == "," and depth == 0:
            parts.append(cur)
            cur = ""
        else:
            cur += ch
    if cur.strip():
        parts.append(cur)
    return [p.strip() for p in parts if p.strip()]


def parse_decls(block: str):
    """Yield (name, [(arg_name, arg_type)], return_type, is_variadic)."""
    # Strip line comments so doc comments cannot confuse the parser.
    text = re.sub(r"//[^\n]*", "", block)
    for m in re.finditer(r"pub fn (\w+)\(", text):
        name = m.group(1)
        i = m.end()
        depth = 1
        j = i
        while depth:
            if text[j] == "(":
                depth += 1
            elif text[j] == ")":
                depth -= 1
            j += 1
        args_text = text[i : j - 1]
        rest = text[j:]
        end = rest.index(";")
        ret = rest[:end].strip()
        ret = ret[2:].strip() if ret.startswith("->") else ""
        variadic = False
        args = []
        for a in split_top_level(args_text):
            if a == "...":
                variadic = True
                continue
            an, at = a.split(":", 1)
            args.append((an.strip(), at.strip()))
        yield name, args, ret, variadic


def rust_ident(name: str) -> str:
    return name.upper()


def main() -> None:
    order = read_order()
    slot = {n: i for i, n in enumerate(order)}
    src = LIB_RS.read_text()
    decls = list(parse_decls(extern_block(src)))

    out: list[str] = []
    w = out.append
    w("// @generated by tools/gen_win_dyn.py -- DO NOT EDIT BY HAND.")
    w("//")
    w("// Inputs: tools/otp27_nif_api_order.txt (the Windows callback-table order of")
    w("// Erlang/OTP 27, ERTS 15.2.7.8) and the `extern \"C\"` block in lib.rs.")
    w("// See tools/gen_win_dyn.py for the full explanation of why this file exists.")
    w("//")
    w("// Everything below the `idx` module is Windows-only: on Linux and macOS the")
    w("// BEAM resolves `enif_*` itself and lib.rs keeps the plain `extern \"C\"` block.")
    w("")
    w("/// Number of function-pointer slots before `erts_alc_test` in OTP 27's table.")
    w(f"pub const TABLE_LEN: usize = {len(order)};")
    w("")
    w("/// The table order, by name.  Slot `i` of the BEAM's callback table holds the")
    w("/// function called `API_NAMES[i]`.  Compiled on every platform so a unit test")
    w("/// can pin it on Linux, where no Windows build is available.")
    w("pub const API_NAMES: [&str; TABLE_LEN] = [")
    for n in order:
        w(f'    "{n}",')
    w("];")
    w("")
    w("/// Slot index of each function in the callback table.")
    w("pub mod idx {")
    for n in order:
        w(f"    pub const {rust_ident(n)}: usize = {slot[n]};")
    w("}")
    w("")
    w("#[cfg(windows)]")
    w("mod imp {")
    w("    use super::{idx, TABLE_LEN};")
    w("    use crate::*;")
    w("    use std::cell::UnsafeCell;")
    w("    use std::ffi::{c_char, c_int, c_long, c_uint, c_void};")
    w("")
    w("    /// The table OTP passes to `nif_init`: `TABLE_LEN` function pointers in")
    w("    /// header order, then `void* erts_alc_test`.  `#[repr(C)]` makes the Rust")
    w("    /// layout identical to the C struct (all fields are pointer-sized).")
    w("    #[repr(C)]")
    w("    pub struct TWinDynNifCallbacks {")
    w("        pub entries: [*const c_void; TABLE_LEN],")
    w("        pub erts_alc_test: *const c_void,")
    w("    }")
    w("")
    w("    /// Our private copy of the table (the C macro memcpy's it the same way).")
    w("    struct Table(UnsafeCell<[*const c_void; TABLE_LEN + 1]>);")
    w("    // SAFETY: written once by `win_init` while the BEAM loads the library (before")
    w("    // any NIF can run), read-only afterwards.")
    w("    unsafe impl Sync for Table {}")
    w("    static TABLE: Table = Table(UnsafeCell::new([std::ptr::null(); TABLE_LEN + 1]));")
    w("")
    w("    /// Store the callback table the BEAM passes to `nif_init`.")
    w("    ///")
    w("    /// # Safety")
    w("    /// `callbacks` must point to the BEAM's `TWinDynNifCallbacks` (at least")
    w("    /// `TABLE_LEN + 1` pointers, i.e. OTP 27 or newer).")
    w("    pub unsafe fn win_init(callbacks: *const TWinDynNifCallbacks) {")
    w("        std::ptr::copy_nonoverlapping(")
    w("            callbacks as *const *const c_void,")
    w("            TABLE.0.get() as *mut *const c_void,")
    w("            TABLE_LEN + 1,")
    w("        );")
    w("    }")
    w("")
    w("    #[inline]")
    w("    unsafe fn slot(i: usize) -> *const c_void {")
    w("        let p = (*TABLE.0.get())[i];")
    w("        assert!(")
    w("            !p.is_null(),")
    w('            "erl-nif-bridge: NIF API slot {} is empty; nif_init must call win_init(callbacks) first",')
    w("            i")
    w("        );")
    w("        p")
    w("    }")
    w("")
    skipped = []

    def wrapper(name, args_sig, arg_types, arg_names, ret):
        retclause = f" -> {ret}" if ret else ""
        w(f"    pub unsafe fn {name}({args_sig}){retclause} {{")
        w(f"        let f: unsafe extern \"C\" fn({', '.join(arg_types)}){retclause} =")
        w(f"            std::mem::transmute(slot(idx::{rust_ident(name)}));")
        w(f"        f({', '.join(arg_names)})")
        w("    }")
        w("")

    for name, args, ret, variadic in decls:
        if name not in slot:
            skipped.append(name)
            continue
        if variadic:
            skipped.append(name + " (variadic)")
            continue
        sig = ", ".join(f"{a}: {t}" for a, t in args)
        wrapper(name, sig, [t for _, t in args], [a for a, _ in args], ret)
    for name, sig, ret in EXTRA:
        if name not in slot:
            sys.exit(f"EXTRA function {name} is not in the table order")
        parts = [p.strip() for p in sig.split(",")]
        names = [p.split(":")[0].strip() for p in parts]
        types = [p.split(":", 1)[1].strip() for p in parts]
        wrapper(name, sig, types, names, ret)

    w("    /// `enif_make_pid` is a header macro on Windows, not a table entry:")
    w("    /// `#define enif_make_pid(ENV, PID) ((const ERL_NIF_TERM)((PID)->pid))`.")
    w("    pub unsafe fn enif_make_pid(_env: ErlNifEnv, pid: *const ErlNifPid) -> ERL_NIF_TERM {")
    w("        (*pid).pid")
    w("    }")
    w("}")
    w("")
    w("#[cfg(windows)]")
    w("pub use imp::*;")
    w("")
    w("// Not wrapped on Windows: " + ", ".join(skipped) + ".")
    OUT.write_text("\n".join(out) + "\n")
    print(f"wrote {OUT.relative_to(PKG)}: {len(order)} slots, "
          f"{len(decls) - len(skipped)} wrappers + {len(EXTRA)} extra; skipped: {skipped}")


if __name__ == "__main__":
    main()
