# sir-refuse-decorators — Refuse Function and Method Decorators with Positioned Errors

## Status

Active design & implementation specification. Addresses item 6 of #18496 and item 1 of #18482.

## Motivation

In Python, decorator syntax (`@decorator`) can precede `class` statements, top-level `def`
statements, nested `def` statements, and method `def` statements inside class bodies:

```python
# Function decorator
@my_decorator
def greet(name):
    return "Hello " + name

# Class method decorator
class Counter:
    @property
    def value(self):
        return 42
```

In `python-to-semantic-ir`, `lower_class` already verified and rejected decorated classes:

```rust
if child_nodes(class_stmt).iter().any(|n| n.rule_name == "decorator") {
    return Err(self.err_at(class_stmt, "unsupported: decorated class (deferred)"));
}
```

However:
1. Top-level and nested function definitions (`def_stmt`) were collected in the first pass
   (`collect_function_names`) and lowered in `lower_def` without checking for `decorator` child
   nodes.
2. Method definitions inside class bodies (`lower_class` / `lower_method_registration`) inspected
   the `def_stmt` node without checking for `decorator` child nodes.

Because Python's grammar rule for function and method definitions is:
```text
def_stmt = decorator* "def" NAME "(" parameters? ")" ":" suite ;
```
the lowerer silently ignored and dropped any leading `decorator` nodes, emitting undecorated
functions. This silent drop leads to incorrect program execution (e.g. `@staticmethod` or
custom wrappers not being applied).

Per repository rules and the security review findings in #18482 and #18496, unsupported language
constructs must always be refused with positioned errors rather than silently ignored or dropped.

## Contract Specification

### 1. Refusing Function Decorators

In `python-to-semantic-ir/src/lower.rs`:
- In `collect_function_names` (and defensively in `lower_def`), inspect `child_nodes(def)`.
- If any child node has `rule_name == "decorator"`:
  Emit a positioned error:
  `"unsupported: decorated function (deferred)"`
  anchored at the decorator node (or the `def_stmt`).

### 2. Refusing Method Decorators

In `python-to-semantic-ir/src/lower.rs`:
- In `lower_class`, when iterating over `class_body_statements(suite)`:
  After obtaining `def = self.as_def_stmt(stmt_node)`:
  If any child node has `rule_name == "decorator"`:
  Emit a positioned error:
  `"unsupported: decorated method (deferred)"`
  anchored at the decorator node (or the `def_stmt`).

### 3. Verification

- Add unit tests in `python-to-semantic-ir/tests/statement_forms.rs`:
  - `decorated_function_is_refused`: test `@dec\ndef f(): pass\n` produces an error containing `"decorated function"`.
  - `decorated_nested_function_is_refused`: test nested `@dec\ndef g(): pass\n` produces an error containing `"decorated function"`.
  - `decorated_method_is_refused`: test `class C:\n    @dec\n    def m(self): pass\n` produces an error containing `"decorated method"`.
- Verify existing tests continue to pass and `clippy` passes with zero warnings.
