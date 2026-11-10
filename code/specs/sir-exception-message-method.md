# sir-exception-message-method — Built-in `message` Method on Exception Objects

## Status

Active design & implementation specification. Addresses item 8 of #18798.

## Motivation

In Ruby and related dynamic object/exception systems, exception objects provide a built-in
zero-argument `.message` method returning the exception's message string (or the exception
class name when no message is provided):

```ruby
def e
  1
end
begin
  raise "boom"
rescue => e
  puts e.message
end
```

When this Ruby program is lowered to Semantic IR (SIR), the exception binding `e` is referenced
as the receiver of method `message`: `__method__(e, "message")`.

In conformance tests (`code/packages/rust/sir-conformance/tests/ruby_sir_gaps.rs`), the test case
`rb_rescue_arrow_binding_named_like_a_method` verified this behavior across backends. Python,
JavaScript, TypeScript, Go, and Rust already supported `e.message`. However, C and Ruby backends
did not support `message` on exception values and were recorded in the `REFUSED` table:

```rust
const REFUSED: &[(&str, Target, &str)] = &[
    (
        "rb_rescue_arrow_binding_named_like_a_method",
        Target::C,
        "the built-in method `message` (Collections batch)",
    ),
    (
        "rb_rescue_arrow_binding_named_like_a_method",
        Target::Ruby,
        "the built-in method `message` (Collections batch)",
    ),
];
```

## Problem Statement

### C Backend (`semantic-ir-to-c`)

1. During compile-time scan in `code/packages/rust/semantic-ir-to-c/src/emit.rs`, `__method__`
   dispatch checks whether the method name is known via `DEFINED_METHODS` or `is_builtin_method(name)`.
   Because `"message"` was missing from `is_builtin_method`, calling `e.message` caused compile-time
   rejection:
   `"a call to the built-in method 'message' (not lowered by the C backend yet)"`.
2. In the C runtime (`code/packages/rust/semantic-ir-to-c/src/runtime.rs`), `_sir_builtin_method_v`
   dispatches built-in method calls. It lacked a dispatch arm for `"message"`. For an exception
   receiver (`recv.tag == SIR_ERROR`), `message` should return `recv.as.err->msg` (or
   `_sir_str(recv.as.err->sir_class)` if `recv.as.err->msg.tag == SIR_NIL`).

### Ruby Backend (`semantic-ir-to-ruby`)

1. In `code/packages/rust/semantic-ir-to-ruby/src/emit.rs`, `__method__` dispatch checks
   `collection_access(args)`. If `collection_access` returns `None` and the method is not a
   user-registered method, emission is rejected at compile time:
   `"a call to the built-in method 'message' (only user-defined methods dispatch this slice; built-in methods are the Collections batch)"`.
   Furthermore, if bypassed, it emitted `(recv).public_send(:sir_um_message)`, which fails on native
   Ruby `Exception` instances since native methods do not have the `sir_um_` prefix.
2. In `collection_access(args)`, `("message", 2)` should map to `"sir_message"`.
3. In `code/packages/rust/semantic-ir-to-ruby/src/runtime.rs`, a runtime helper `def sir_message(recv)`
   must be provided:
   - If `recv` responds to `:sir_um_message` (user class instance method), dispatch `recv.public_send(:sir_um_message)`.
   - If `recv` is an `Exception`, return `recv.message`.
   - Otherwise, raise `NoMethodError`.

### Conformance

With both C and Ruby backends supporting `message`, `rb_rescue_arrow_binding_named_like_a_method`
passes on all backends. The `REFUSED` table in `ruby_sir_gaps.rs` becomes empty (`&[]`).

## Implementation Plan

1. Add `code/specs/sir-exception-message-method.md` and commit with:
   `docs(specs): specify exception message method on C and Ruby backends (#18798)`
2. Update `semantic-ir-to-c`:
   - In `emit.rs`: add `"message"` to `is_builtin_method`.
   - In `runtime.rs`: in `_sir_builtin_method_v`, add handling for `strcmp(m, "message") == 0 && argc == 0`:
     ```c
     else if (strcmp(m, "message") == 0 && argc == 0) {
         if (recv.tag == SIR_ERROR) {
             if (recv.as.err->msg.tag == SIR_NIL) {
                 return _sir_str(recv.as.err->sir_class);
             }
             return recv.as.err->msg;
         }
     }
     ```
3. Update `semantic-ir-to-ruby`:
   - In `emit.rs`: in `collection_access`, add `("message", 2) => Some("sir_message")`.
   - In `runtime.rs`: add `sir_message(recv)` helper.
4. Update `sir-conformance`:
   - In `tests/ruby_sir_gaps.rs`: empty `REFUSED`.
5. Update `CHANGELOG.md` in `semantic-ir-to-c`, `semantic-ir-to-ruby`, and `sir-conformance`.
6. Run tests and clippy across all touched packages.
