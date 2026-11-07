# sir-list-mutations — List aliasing & in-place mutations across SIR backends

## Status

Active design & implementation specification. Addresses item 4 of #18496
(SIR open issues triage, follow-up to #18482 and #18490).

## Motivation

In Python and Ruby, lists/arrays are mutable reference types. When two variables
reference the same list (aliasing), in-place operations performed through one reference
must be immediately observable through all other aliases:

```python
xs = [1, 2]
ys = xs

# 1. Method mutation via .append()
xs.append(3)
# ys must observe [1, 2, 3]

# 2. Augmented assignment via +=
xs += [4, 5]
# ys must observe [1, 2, 3, 4, 5]
```

Prior to this work, three concrete defects prevented aliased mutations from functioning
correctly across SIR frontends and backends:

1. **JavaScript backend (`semantic-ir-to-javascript`)**:
   `arrayMethod` in `runtime.rs` did not handle `"append"`. When dispatching `xs.append(v)`
   via `__method__`, it missed the array catalog and fell through to the security allowlist.
   Because `METHOD_ALLOWLIST` only contained `"push"`, calling `.append` raised a runtime
   `NoMethodError: undefined method append for an instance of Array`. Furthermore,
   native JS `Array.prototype.push` returns the new array length rather than the array
   reference, which broke chaining and return value expectations.

2. **C backend (`semantic-ir-to-c`)**:
   `is_builtin_method` in `emit.rs` checked for `"push"` but lacked `"append"`, causing the
   compiler to reject the call as `a call to the built-in method append (not lowered by the C backend yet)`.
   Additionally, `_sir_builtin_method` in `runtime.rs` only matched `strcmp(m, "push") == 0`.

3. **Ruby backend (`semantic-ir-to-ruby`)**:
   `collection_access` in `emit.rs` only matched `"[]"`, `"[]="`, and `"size"` / `"length"`.
   All other `__method__` calls were lowered to `(recv).public_send(:sir_um_<method>)`. Because
   Ruby's native `Array` does not define `sir_um_append`, calling `.append` raised `NoMethodError`.

4. **In-place `+=` lowered to rebinding assignment (`python-to-semantic-ir` & backends)**:
   In `python-to-semantic-ir/src/lower.rs`, `lower_augmented` stripped `=` from `+=` and
   lowered `x += y` to `Assign { name: x, value: BuiltinCall("+", [x, y]) }`. Because binary
   `+` produces a brand new sequence, `x` was rebound to a new list object while aliases like `ys`
   continued pointing to the unmodified original list.

## Contract Specification

### 1. In-place `.append` method semantics

When a collection method call `BuiltinCall("__method__", [receiver, "append", ...args])` is
evaluated:

- **Array receiver**: Appends all items in `args` to `receiver` in place.
- **Return value**: Returns the modified `receiver` (matching `push` / `append` chaining convention).
- **Aliases**: Any aliases sharing the same sequence reference immediately observe the new elements.
- **Backend implementations**:
  - **JS**: In `arrayMethod`, add `case "push": case "append":` which executes `recv.push(...args)`
    and returns `recv`. Add `"append"` to `ARRAY_METHODS` and `METHOD_ALLOWLIST`.
  - **C**: In `emit.rs`, add `"append"` to `is_builtin_method`. In `runtime.rs`, update
    `_sir_builtin_method` to match `strcmp(m, "push") == 0 || strcmp(m, "append") == 0`.
  - **Ruby**: In `emit.rs`, map `"append"` and `"push"` in `collection_access` to `sir_append`.
    In `runtime.rs`, implement `def sir_append(recv, *args)` that appends to `Array` via
    `recv.push(*args); recv` or delegates to user method `sir_um_append` / `sir_um_push`.
  - **Go, Rust, Python, TS**: Already support `"push"` and `"append"` in their collection runtime.

### 2. Augmented `+=` Builtin (`BuiltinCall("+=", [lhs, rhs])`)

In `python-to-semantic-ir`:
- `lower_augmented` maps `"+="` to `BuiltinCall("+=", [current, rhs])` instead of binary `"+"`.
- Other augmented operators (`-=`, `*=`, `/=`, `%=`) continue mapping to their respective binary
  arithmetic builtins because numbers and strings are immutable values where rebinding is semantically
  identical to in-place mutation.

Across all backends, `"+="` routes to an in-place addition helper (`iadd`):
- **If `lhs` is a sequence/array**:
  - Appends elements of `rhs` to `lhs` in place.
  - Returns `lhs` (the same handle/pointer/reference).
  - The outer assignment re-stores `lhs` into the target variable, preserving object identity across all aliases.
- **If `lhs` is numeric or string or other primitive**:
  - Delegates to standard binary addition (`add(lhs, rhs)`).
  - Returns the resulting value.

### Backend Details for `+=`

| Backend | Runtime Helper | Sequence In-place Action |
|---|---|---|
| **JavaScript** | `__Sir.iadd(a, b)` | `if (Array.isArray(a)) { a.push(...b); return a; }` |
| **TypeScript** | `sir.iadd(a, b)` in `sir-runtime-core` | `if (Array.isArray(a)) { a.push(...b); return a; }` |
| **Python** | `sir.iadd(a, b)` in `sir-runtime-core` | `if isinstance(a, list): a.extend(b); return a` |
| **C** | `_sir_iadd(a, b)` in C runtime | Append items of `b.as.seq` to `a.as.seq`, return `a` |
| **Go** | `_sir_iadd(a, b)` in Go runtime | `aSeq.Items = append(aSeq.Items, bSeq.Items...); return aSeq` |
| **Rust** | `__sir::iadd(a, b)` in Rust runtime | `a_items.borrow_mut().extend(b_items.borrow().iter().cloned()); return a` |
| **Ruby** | `sir_iadd(a, b)` in Ruby runtime | `if a.is_a?(Array) && b.is_a?(Array) { a.concat(b); a } else { a + b }` |

## Verification Plan

1. **Frontend Lowering Unit Tests**:
   - `python-to-semantic-ir`: verify `x += y` lowers to `BuiltinCall("+=", ...)` and `x.append(y)`
     lowers to `BuiltinCall("__method__", [x, "append", y])`.
2. **Backend Execution Tests**:
   - JS, C, Ruby, Go, Rust, TS, Python: test that `xs = [1]; ys = xs; xs.append(2)` results in `ys == [1, 2]`.
   - Test that `xs = [1]; ys = xs; xs += [2]` results in `ys == [1, 2]`.
   - Test that `n = 1; m = n; n += 2` preserves `m == 1` and `n == 3` (primitives not mutated).
3. **Conformance Integration Tests**:
   - In `sir-conformance`: add end-to-end Python statement forms tests verifying list aliasing under
     both `.append` and `+=`.
