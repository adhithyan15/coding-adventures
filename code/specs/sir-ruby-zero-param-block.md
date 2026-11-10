# Specification: Block with No Parameters on Ruby Backend

**Issue**: #18798 (Item 23)  
**Package**: `semantic-ir-to-ruby`  
**Status**: In Implementation  

---

## 1. Problem Statement

In standard Ruby (MRI), blocks passed to methods or iterators have `Proc` semantics rather than strict `lambda` semantics. Specifically:
- Extra arguments yielded to a block that declares fewer parameters are silently ignored and discarded.
- In particular, a block declaring no parameters (`{ puts 42 }` or `do puts 42 end`) accepts any number of arguments yielded by the caller (such as `[1, 2].each`, `{a: 1}.each`, or a user method yielding values `yield 1, 2`).

```ruby
xs = [1, 2]
xs.each { puts 42 }
```

Expected output:
```text
42
42
```

However, on the Ruby backend (`semantic-ir-to-ruby`), executing this program fails with:
```text
ArgumentError: wrong number of arguments (given 1, expected 0)
```

### Root Causes

1. **`sir_each` in `runtime.rs`**:
   In `sir_each(recv, blk, arity = nil)`:
   - When `recv` is an `Array`, `sir_each` unconditionally executes `recv.each { |x| blk.call(x) }`, always passing `x` regardless of `arity`.
   - When `recv` is a `Hash`, if `arity == 0`, `sir_each` falls into the `else` branch and executes `recv.each { |k, v| blk.call(k, v) }`, passing two arguments.
   When `arity == 0`, `sir_each` should invoke `blk.call` without arguments:
   `recv.each { blk.call }`.

2. **Closure Emission in `emit.rs`**:
   The Ruby frontend lowers a 0-parameter block to an anonymous function with `params: []` and constructs an `Expr::MakeClosure { fn_name, captures, ... }`.
   In `semantic-ir-to-ruby/src/emit.rs`, `Expr::MakeClosure` emits a forwarding lambda:
   ```ruby
   ->(*sir_args) { sir_fn___block_0(captures..., *sir_args) }
   ```
   Even when `fn_name` has declared exact arity 0 (represented by `STRICT_ARITY` mapping `fn_name -> 0`), `*sir_args` is forwarded directly to `sir_fn___block_0`. Because `sir_fn___block_0` is defined as a standard Ruby method (`def sir_fn___block_0(...)`), Ruby enforces exact method arity at the call site.
   If a caller yields any arguments (e.g. `yield 1` or `blk.call(x)`), `*sir_args` is non-empty, causing `sir_fn___block_0` to raise:
   `ArgumentError: wrong number of arguments (given N, expected 0)`.

---

## 2. Technical Design

### 2.1 Runtime: `sir_each` in `semantic-ir-to-ruby/src/runtime.rs`

Update `sir_each` to check `arity == 0` for both `Array` and `Hash`:

```ruby
def sir_each(recv, blk, arity = nil)
  return recv.public_send(:sir_um_each, blk) if recv.respond_to?(:sir_um_each)

  case recv
  when Array
    if arity == 0
      recv.each { blk.call }
    else
      recv.each { |x| blk.call(x) }
    end
  when Hash
    if arity == 1
      recv.each { |k, v| blk.call([k, v]) }
    elsif arity == 0
      recv.each { blk.call }
    else
      recv.each { |k, v| blk.call(k, v) }
    end
  else raise NoMethodError, "undefined method 'each' for #{recv.inspect}"
  end
  recv
end
```

### 2.2 Emitter: `Expr::MakeClosure` in `semantic-ir-to-ruby/src/emit.rs`

When emitting an unmarked `Expr::MakeClosure` (not wrapped in `BuiltinCall("lambda", ...)` which denotes strict lambda arity checking):
Consult `STRICT_ARITY` for `fn_name`. If `STRICT_ARITY` is `Some(0)`:
The target function expects 0 parameters (apart from captures). The forwarding lambda should discard all passed arguments:
```rust
let arity = STRICT_ARITY.with(|t| t.borrow().get(fn_name).copied());
if arity != Some(0) {
    fixed.push("*sir_args".to_string());
}
```
This produces:
```ruby
->(*sir_args) { sir_fn___block_0(captures...) }
```
Any call `blk.call(...)` with 0, 1, or more arguments invokes `sir_fn___block_0(captures...)` without passing `*sir_args`, perfectly matching Ruby block parameter semantics where surplus arguments are ignored.

For closures with `arity != Some(0)` (including 1-parameter and multi-parameter closures, and functions with dynamic arity), `*sir_args` continues to be forwarded as before, preserving strict signature matching and existing test assertions.

---

## 3. Testing & Verification

1. **Unit Tests in `semantic-ir-to-ruby/tests/emit_tests.rs`**:
   - `[1, 2].each { puts 42 }` runs over an array with a 0-parameter block, asserting output `"42\n42"` and generated Ruby containing `sir_each(..., 0)`.
   - `{a: 1, b: 2}.each { puts 99 }` runs over a hash with a 0-parameter block, asserting output `"99\n99"`.
   - Yielding to a 0-parameter block from a user method (`def foo; yield 1; end; foo { puts 10 }`) succeeds and prints `10`.

2. **Conformance Tests in `sir-conformance/tests/ruby_sir_gaps.rs`**:
   - Add conformance entry verifying `[1, 2].each { puts 42 }` and user method yielding to a 0-parameter block.

3. **Validation**:
   - All tests in `semantic-ir-to-ruby` and `sir-conformance` pass.
   - Zero compiler and Clippy warnings (`cargo clippy --all-targets -- -D warnings`).
