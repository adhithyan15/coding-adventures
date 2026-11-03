---
category: Rust
---

# Putting Rc inside a public value enum silently removes Send and Sync from every embedder that stores it

**What went wrong.** LANG83's PR5b plan said vm-core's `Value::Str` should
become `Rc<[u8]>`. `vm-core` itself compiled. `jit-core` did not. Its tests,
like many embedders, record argument values from inside a builtin callback
into an `Arc<Mutex<Vec<Value>>>`, and builtins must be `Send + Sync`
(`register_builtin`, `BuiltinRegistry::register`). With an `Rc` inside, `Value`
was neither, and every such closure stopped compiling (E0277 "`Rc<[u8]>`
cannot be sent between threads safely").

**Fix.** `Value::Str(Arc<[u8]>)`. The count changes only when a register is
copied, so the atomic increment costs nothing measurable. The spec records the
divergence.

**Do differently.** Before you change a field type in a public enum, check
the auto traits it had (`Send`, `Sync`, `Unpin`, `UnwindSafe`). They are part
of the API even though no signature names them. Run `cargo check
--all-targets` on the reverse dependencies, not only on the crate that owns the
type: the breakage shows up in the embedder, not in the crate you changed.
