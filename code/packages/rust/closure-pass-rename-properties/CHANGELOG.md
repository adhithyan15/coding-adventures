# Changelog

All notable changes to the `coding-adventures-closure-pass-rename-properties` crate will be documented in this file.

## [0.2.0] - 2026-06-18

### Added (CLOC13.K — `collect_property_names`, the externs property boundary)

A public function `collect_property_names(program) -> HashSet<String>` that
returns **every property name appearing anywhere** in a program — dotted member
accesses (`el.innerHTML`), quoted member accesses (`obj["data-id"]`), unquoted
object keys (`{ onload: f }`), and quoted object keys (`{ "aria-label": s }`).

This is the property-namespace analogue of collecting an externs file's
top-level variable/function names (the value-namespace boundary). A driver
(closurec) walks each `--externs` file through this function and unions the
results into the `do_not_rename` set it hands `RenamePropertiesPass::new`, so the
external host/library property surface is preserved while program-private
properties are still shortened.

- **Over-collects on purpose.** Both renameable (dotted) and off-limits (quoted)
  occurrences are returned: as an externs boundary, every named property is
  external and must be protected. Forgoing a rename is never a miscompile;
  renaming a genuinely external property is. Dynamic computed keys
  (`obj[runtimeExpr]`) contribute nothing — there is no static name to protect.
- Reuses the pass's existing whole-program `classify_item` walk (no second
  traversal implementation to keep in sync).
- 9 new unit tests + 1 doctest covering each occurrence shape, dynamic-key
  exclusion, function-body recursion, and an end-to-end "collected externs
  protect a property" round-trip.

## [0.1.0] - 2026-06-18

### Added (CLOC13.J — aggressive property renaming, algorithmic core)

New crate per CLOC06's canonical pass set — Closure Compiler's `RENAME_PROPERTIES`
in miniature. `RenamePropertiesPass::run` consistently shortens program-private
object **property names** across the whole program (every dotted `obj.x` member
access and every unquoted `{ x: … }` key of a renameable name → a fresh short
name). Property access is by name, so renaming a name at every occurrence is
semantics-preserving regardless of which objects carry it.

- **ADVANCED-only, sound under the externs contract.** A property name is
  renamed only when it: appears dotted/unquoted; is NOT quoted via a computed
  string member (`obj["x"]` — the bridge preserves this signal); is not a
  `BUILTIN_PROPERTIES` (a bundled ECMAScript default-externs substitute —
  `length`, `prototype`, `toString`, `push`, …); is not in the externs
  do-not-rename set; and is longer than one character. Each renameable property
  gets a distinct fresh name. Property names live in their own namespace, so the
  fresh name only avoids other property names + the built-ins + the externs set.
- **Honest limitations (documented in the crate):**
  - The built-in list covers ECMAScript but NOT the DOM/host — host property
    names (`innerHTML`, `addEventListener`, …) must be supplied via `--externs`.
  - The parser bridge currently collapses a *quoted object key* `{ "x": 1 }` to
    an identifier key, so object-key quoting is not a usable do-not-rename
    signal (only computed-member quoting `obj["x"]` is); protect such names via
    externs. (A separate bridge fix is tracked.)
  - Dynamic computed access `obj[runtimeString]` is the author's contract
    responsibility, exactly as in Closure.
- `name = "rename-properties"`, `depends_on = []`, `iteration_policy = OneShot`,
  `cost = 3`. `new(do_not_rename)` / `with_builtins_only()`.

This is the algorithmic core; wiring into ADVANCED (collecting externs property
names + deciding the safe-by-default policy — require externs / bundle DOM
externs) is a deliberate follow-up.

### Tests
- 13 tests: metadata contract + source → bridge → rename-properties → emit
  roundtrips covering consistent dotted+object-key renaming, computed-member
  quoting decline, built-in protection, externs protection, dynamic computed
  key, single-char skip, and a nested property chain.
