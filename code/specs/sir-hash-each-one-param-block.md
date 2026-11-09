# Specification: Hash#each with a One-Parameter Block Across Backends

**Issue**: #18798 (Comment 1)  
**Packages**: `semantic-ir-to-python`, `sir-runtime-oop` (Python & TS), `semantic-ir-to-javascript`, `semantic-ir-to-go`, `semantic-ir-to-rust`, `semantic-ir-to-c`  
**Status**: In Implementation  

---

## 1. Problem Statement

In standard Ruby (MRI), `Hash#each` and its alias `Hash#each_pair` adapt yielded values to the block's declared arity:

```ruby
{a: 1, b: 2}.each do |pair|
  for x in pair
    puts x
  end
end
```

Expected output:
```text
a
1
b
2
```

1. **One-parameter block (`|pair|`)**:
   Ruby gives the block a 2-element array `[key, value]`. In the example above, `pair` binds `[:a, 1]` on the first iteration and `[:b, 2]` on the second. The `for x in pair` loop iterates over the 2-element array and prints `a` followed by `1`.

2. **Two-parameter block (`|k, v|`)**:
   Ruby gives the block two separate arguments: `k = :a`, `v = 1`.

3. **Current status across backends**:
   - **Ruby backend** (`semantic-ir-to-ruby`): Fixed in #18778 — `sir_each` receives `arity` from `exact_arity` and yields `[k, v]` if `arity == 1`.
   - **Python backend**: `_hash_block_method` unconditionally calls `apply(block, [key, value])`. With `block.arity == 1`, `apply` truncates the arguments to `[key]`, so `pair` binds `:a` (a Symbol). Attempting to iterate `pair` in `for v in pair` raises `TypeError: 'Symbol' object is not iterable`.
   - **TypeScript / JavaScript backend**: `hashBlockMethod` / JS runtime unconditionally calls `apply(block, [k, v])` or `blk(k, v)`. The underlying function takes 1 parameter, so `pair` receives only `k`. Attempting to iterate `pair` throws `TypeError: pair is not iterable`.
   - **Go backend**: `_sir_hash_block_method` passes `[]Value{e.Key, e.Val}`. The block thunk splats into the single `pair` parameter, receiving only `e.Key`. Attempting to iterate `pair` panics: `cannot iterate non-sequence: a`.
   - **Rust backend**: `call_method` calls `apply_closure(b, vec![k, v])`. The closure iterator extracts only `k` into `pair`. The `for` loop checks for `Value::Seq`, finds `Value::Sym`, and panics: `cannot iterate non-sequence: a`.
   - **C backend**: `_sir_hash_each` calls `_sir_apply(block, 2, entries[i].key, entries[i].val)`. The thunk supplies `key` to `pair`. The `for` loop checks `_sir_is_seq(pair)` (false) and does 0 iterations, silently printing nothing.

---

## 2. Technical Architecture & Design

To resolve this across all backends with complete consistency:

### 2.1 Python (`coding-adventures-sir-runtime-oop`)
In `_hash_block_method`:
Check `getattr(block, "arity", None)`:
```python
if name in ("each", "each_pair"):
    for key, value in list(recv.items()):
        if getattr(block, "arity", None) == 1:
            apply(block, [[key, value]])
        else:
            apply(block, [key, value])
    return recv
```
Because `Closure` in `sir-runtime-core` already computes `self.arity = max(0, arity - len(captures))` from `_positional_arity`, a block with `|pair|` has `block.arity == 1`. Passing `[[key, value]]` provides 1 argument consisting of the 2-element list `[key, value]`.

### 2.2 TypeScript / JavaScript (`sir-runtime-oop` & `semantic-ir-to-javascript`)
1. In `sir-runtime-core` (TypeScript), allow `Closure` to record `arity?: number`. In `makeClosure(fn, captures, arity?)`: store `arity`.
2. In `semantic-ir-to-typescript`, pass `f.params.len()` to `new __Sir.Closure(...)`.
3. In `sir-runtime-oop` (`index.ts`):
   ```typescript
   case "each":
   case "each_pair":
     for (const [k, v] of [...recv]) {
       if (block && (block as any).arity === 1) {
         apply(block, [[k, v]]);
       } else {
         apply(block, [k, v]);
       }
     }
     return recv;
   ```
4. In `semantic-ir-to-javascript` runtime:
   Record `arity` on `Closure`. In `case "each": case "each_pair":`:
   If `blk.arity === 1`, pass `[k, v]` as a single array argument.

### 2.3 Go (`semantic-ir-to-go`)
1. In `semantic-ir-to-go/src/runtime.rs`:
   Add `Arity int` to `type Closure struct { Fn func(args []Value) Value; Arity int }`.
   `_sir_make_closure(fn, captures, arity int) Value`.
2. In `semantic-ir-to-go/src/emit.rs`:
   Pass `params_arity` (already computed via `FN_ARITY`) to `_sir_make_closure`.
3. In `_sir_hash_block_method`:
   ```go
   case "each", "each_pair":
       for _, e := range recv.Entries {
           if block != nil && block.Arity == 1 {
               _sir_apply(block, []Value{&Seq{Items: []Value{e.Key, e.Val}}})
           } else {
               _sir_apply(block, []Value{e.Key, e.Val})
           }
       }
       return recv, true
   ```

### 2.4 Rust (`semantic-ir-to-rust`)
1. In `semantic-ir-to-rust/src/runtime.rs`:
   Add `pub arity: usize` to `struct Closure`.
2. In `semantic-ir-to-rust/src/emit.rs`:
   In `emit_make_closure`, initialize `arity: {arity}` using `FN_ARITY` (already present).
3. In `runtime.rs`:
   ```rust
   "each" | "each_pair" => {
       if let Some(b) = &block {
           for (k, v) in entries_rc.borrow().clone() {
               if b.arity == 1 {
                   apply_closure(b, vec![seq_lit(vec![k, v])]);
               } else {
                   apply_closure(b, vec![k, v]);
               }
           }
       }
       recv
   }
   ```

### 2.5 C (`semantic-ir-to-c`)
1. In `semantic-ir-to-c/src/runtime.rs`:
   Add `int nparam;` to `struct SirClosure`.
   Initialize `c->nparam = nparam;` in `_sir_make_closure(SirFn fn, int ncap, int nparam, ...)`.
2. In `semantic-ir-to-c/src/emit.rs`:
   Pass `f.params.len()` as `nparam` to `_sir_make_closure`.
3. In `_sir_hash_each`:
   ```c
   int nparam = (block.tag == SIR_CLOSURE) ? block.as.clo->nparam : -1;
   for (int64_t i = 0; i < n; i++) {
       if (nparam == 1) {
           _sir_apply(block, 1, _sir_seq_of(2, entries[i].key, entries[i].val));
       } else {
           _sir_apply(block, 2, entries[i].key, entries[i].val);
       }
   }
   ```

---

## 3. Testing and Conformance

Add cross-backend test in `sir-conformance` and individual backend test suites:
```ruby
h = {a: 1, b: 2}
h.each do |pair|
  for x in pair
    puts x
  end
end
```
Expected output:
```text
a
1
b
2
```
Verify two-parameter blocks continue to receive separate keys and values:
```ruby
h = {a: 1}
h.each do |k, v|
  puts k
  puts v
end
```
Expected output:
```text
a
1
```
