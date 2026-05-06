# LS06 — Twig DAP Variables Panel

> **Depends on**: [`LS03`](LS03-dap-adapter-core.md) (`dap-adapter-core`),
> [`05d`](05d-debug-sidecar-format.md) (debug-sidecar binary format),
> [`LANG13`](LANG13-debug-sidecar.md) (sidecar writer API).

## Motivation

After PR #2202 the VS Code → twig-dap → twig-vm path works end-to-end —
breakpoints hit, stepping advances, the call stack shows real frames.
**Except** the Variables panel is empty.  `twig-dap`'s README documents
this gap explicitly:

> Variable declarations are not emitted — Twig's IIR doesn't carry user
> variable names through to register IDs in a way that maps cleanly to
> the sidecar's `(name, reg_index)` shape.

LS06 closes the gap.

## Why the existing slot-index design doesn't work

`debug-sidecar::declare_variable` requires a `reg_index: u32`.  The VM's
debug protocol's `get_slot { slot: N }` returns the value of the Nth
entry in `last_frame_registers`, which is the **alphabetically-sorted
list of live register names** at the current instruction.

That slot index isn't stable as new variables come into scope:

```
Function with registers {bar, foo}:
  instr 5:  live={foo}        → snapshot: [(foo, …)]              → foo = slot 0
  instr 10: live={bar, foo}   → snapshot: [(bar, …), (foo, …)]    → bar = slot 0, foo = slot 1
```

`foo` moves from slot 0 to slot 1.  The sidecar would have to record a
*different* slot per instruction range, which the existing
per-(name, reg_index) declaration shape doesn't support.

## Design — query the VM by name, not by slot

Add a name-keyed query to the VM debug protocol.  The DAP layer's
`handle_variables` queries by name; the slot index in the sidecar
becomes a stable disambiguator (e.g. declaration order in the function),
not a runtime index.

### VM-side change (`twig-vm/src/debug_server.rs`)

New command:

```jsonc
// Editor → VM
{ "cmd": "get_slot_by_name", "name": "bill" }

// VM → editor
{ "kind": "any", "repr": "50" }                  // found
{ "kind": "any", "repr": "<undef>" }             // not in current frame
```

Implementation: linear search through `last_frame_registers` for a
matching name, return the repr.  Cheap — frames have at most a few
dozen registers in practice.

### `dap-adapter-core` change (`vm_conn.rs`)

Add `get_slot_by_name(&mut self, name: &str) -> Result<String, String>`
to the `VmConnection` trait.  Wire it into both `MockVmConnection`
(test stub) and `TcpVmConnection` (sends the new command).

Update `server::handle_variables`: instead of `conn.get_slot(reg_idx)`,
call `conn.get_slot_by_name(&var.name)`.  The sidecar's `reg_index`
field stays in the format for back-compat but is no longer used by the
DAP code path.

### `twig-dap` change (`build_sidecar` in `lib.rs`)

For each `IIRInstr` whose `dest` is a **user-friendly name** (i.e.
doesn't start with `_`), emit:

```rust
w.declare_variable(
    &func.name,
    declaration_idx as u32,    // arbitrary stable id, not used at runtime
    name,                       // ← what handle_variables now queries
    "any",                      // type hint; refine in follow-up
    instr_idx,                  // live_start = first write
    n_instrs,                   // live_end = end of function (SSA never re-binds)
);
```

`declaration_idx` is the position-in-emission counter so the sidecar
format's uniqueness invariant holds.

### Name filtering

User names are everything that doesn't start with `_`.  The IR
compiler uses `_r1`, `_r2`, … for synthesised SSA names; user `define`
and `let` names land verbatim.  This filter keeps the panel signal-rich.

## Top-level `define` — separate scope

`(define x 5)` at the script's top level binds into the script's
`<top>` frame, which has its own `Frame` instance.  Verified by
inspection: top-level forms compile to instructions in a synthesised
top-level function, and that function's HashMap holds the values.
No special handling needed — the sidecar's per-function variable
records cover both cases.

## Test plan

A new integration test in `twig-dap/tests/` runs the tip-calculator
program (or a similar fixture) through the full DAP server, hits a
breakpoint, and asserts:

- `scopes` request returns one "Locals" scope with non-zero
  `variablesReference`
- `variables` request returns rows with `name="bill"`, `value="50"`,
  etc.
- Names starting with `_` are filtered out

Plus unit tests at each layer:

- VM debug server: `get_slot_by_name` for known/unknown names
- `MockVmConnection::get_slot_by_name` round-trip
- `TcpVmConnection::get_slot_by_name` over a real socket
- `build_sidecar`: produces `declare_variable` rows for user names,
  filters synthesised ones

## Out of scope (follow-ups)

- **Type info beyond `"any"`** — would need to thread `type_hint` from
  IIR into `declare_variable`'s `type` argument.
- **Expandable values** — VS Code's Variables panel can expand
  structured values (lists, records, …).  Today `repr` is a string;
  expansion needs nested `variablesReference` plumbing.
- **Watches / `evaluate` request** — the Debug Console's `evaluate`
  isn't wired yet; LS07 candidate.
- **Globals scope** — if top-level bindings ever move from the
  synthesised top frame to a true global, we'd want a separate
  "Globals" scope.

## Stack position

```
              user sets a breakpoint, hits it
                            │
                            ▼
   editor →  scopes/variables DAP requests
                            │
                            ▼
   dap-adapter-core::handle_variables
                            │  reads name from sidecar
                            ▼
   sidecar.live_variables(fn, instr) → [{name, ..}, …]
                            │  for each name
                            ▼
   conn.get_slot_by_name(name)        ← NEW
                            │  TCP
                            ▼
   twig-vm::debug_server "get_slot_by_name"   ← NEW
                            │  HashMap lookup
                            ▼
   {repr: "50"} → DAP `variables` response
                            │
                            ▼
                user sees `bill = 50` in the panel
```
