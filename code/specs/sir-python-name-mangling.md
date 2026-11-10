# SIR Specification: Python Private Name Mangling

## Problem Statement

In Python, identifiers starting with two or more underscores and ending with at most one underscore (`__spam`) that textually appear within a class definition are private names. CPython textually mangles these identifiers by prefixing them with `_ClassName`, where `ClassName` is the current class name with leading underscores stripped (Python Language Reference §6.2.1):

> **Private name mangling**: When an identifier that textually occurs in a class definition begins with two or more underscore characters and ends with at most one underscore character, it is considered a private name of that class. Private names are transformed to a longer form before code is generated for them. The transformation inserts the class name, with leading underscores removed, and the prefix `_` in front of the name. If the class name consists only of underscores, no transformation is done.

Previously, `python-to-semantic-ir` did not implement private name mangling (#18496 Review). This resulted in several silent miscompilations and divergences from CPython:

1. **Bare function calls resolved to outer scopes**:
   In:
   ```python
   def __helper():
       return "outer"
   class C:
       def call_helper(self):
           return __helper()
   ```
   CPython mangles `__helper()` inside `class C` to `_C__helper()`, which fails to find `__helper` and raises `NameError: name '_C__helper' is not defined`. In `python-to-semantic-ir`, it lowered as a `DirectCall` to outer `__helper`.

2. **Private methods were un-mangled**:
   In:
   ```python
   class C:
       def __m(self):
           return 42
       def call_m(self):
           return self.__m()
   ```
   The method was registered as bare `__m`, and called as `__m`, so external callers could invoke `c.__m()` directly and subclasses could unintentionally overwrite `__m`. Under Python semantics, the method is named `_C__m`.

3. **Private instance variables were un-mangled**:
   `self.__x = 10` lowered to `@__x` rather than `@_C__x`.

4. **Private locals and parameters**:
   Local variables and parameters named `__var` inside class methods were not mangled to `_C__var`.

## Transformation Rules

### 1. Mangling Predicate (`should_mangle`)

An identifier `ident` is eligible for private name mangling if:
- `ident.starts_with("__")` (at least two leading underscores)
- `!ident.ends_with("__")` (at most one trailing underscore)
- The enclosing class name, after stripping leading underscores, is non-empty (`!class_name.trim_start_matches('_').is_empty()`).

Examples:
- `__foo` -> mangled to `_ClassName__foo`
- `__foo_` -> mangled to `_ClassName__foo_`
- `___bar` -> mangled to `_ClassName___bar`
- `__init__` -> NOT mangled (ends with `__`)
- `__str__` -> NOT mangled (ends with `__`)
- `_foo` -> NOT mangled (only one leading underscore)
- `__` -> NOT mangled (ends with `__`)

Class name stripping:
- Class `Foo` -> `_Foo__spam`
- Class `_Foo` -> `_Foo__spam` (leading `_` stripped, then prefixed with `_`)
- Class `__Foo` -> `_Foo__spam`
- Class `___` -> NOT mangled (stripped class name is empty)

### 2. Lexical Scope Tracking

`Lowerer` tracks the stack of enclosing classes:
```rust
class_scopes: Vec<String>
```
When entering `lower_class`:
- `self.class_scopes.push(name.clone())`
When leaving `lower_class`:
- `self.class_scopes.pop()`

When classes are nested (`class Outer: class Inner: ...`), the innermost class is used for mangling within its body.

### 3. Application Points

Inside a class scope (`self.current_class_scope().is_some()`):

1. **Method Definitions**:
   In `lower_method_registration`, `def __m(self): ...` mangles `__m` to `_ClassName__m`.
   The hoisted function is named `{ClassName}__{mangled_name}`, and registered via `__def_method__("ClassName", mangled_name, ...)`. Special methods like `__init__` continue mapping to `"initialize"` as before.

2. **Attribute Access and Method Calls**:
   In `SuffixKind::Attr(attr_name)` and `Target::Instance(attr_name)`:
   If `should_mangle(&attr_name)`, transform `attr_name` to `_ClassName__{attr_name}`.
   - `self.__x` becomes `@_ClassName__x`.
   - `self.__m()` becomes method call `_ClassName__m`.
   - `other.__secret()` becomes method call `_ClassName__secret`.

3. **Variable References and Function Calls**:
   When lowering a `NAME` token in expression position (`resolve_var`):
   If `should_mangle(&name)`, transform `name` to `_ClassName__{name}` before checking locals, parameters, or emitting calls.

4. **Assignments and Parameters**:
   In `lower_assignment`, `bind_name`, and parameter declarations within methods of a class:
   If `should_mangle(&name)`, transform `name` to `_ClassName__{name}`.

## Verification & Conformance

- Unit tests in `python-to-semantic-ir`:
  - `class_private_method_mangled`
  - `class_private_instance_var_mangled`
  - `class_outer_helper_not_called_due_to_mangling`
  - `class_with_leading_underscores_mangled`
  - `class_all_underscores_not_mangled`
  - `class_dunder_methods_not_mangled`
- Conformance test in `sir-conformance/tests/python_statement_forms.rs`:
  - `py_class_private_name_mangling` covering private method dispatch, private instance variable isolation across subclassing, and NameError protection across all supported backends.
