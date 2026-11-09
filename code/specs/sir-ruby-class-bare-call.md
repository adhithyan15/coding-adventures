# SIR Spec: Bare-Name Method Calls on Self Inside Class Methods

**Issue:** #18798 Item 21  
**Status:** Approved  
**Affected Crates:** `ruby-to-semantic-ir`

## Background

In Ruby, within an instance method of a class, method invocations without an explicit receiver (e.g. `m`, `m()`, `m(1, 2)`, `setup`) are implicitly invoked on `self`:

```ruby
class C
  def m
    1
  end

  def test
    m
  end
end
```

Prior to this specification, `ruby-to-semantic-ir` handled bare method names using a pre-pass `collect_method_names` that populated:
- `top_level_methods`: methods defined outside any class or module.
- `nested_methods`: methods defined inside any class or module.

In `bare_name_call`:
- If an identifier was shadowed by a local or parameter, it resolved as a variable read.
- If it was in `top_level_methods` and NOT in `nested_methods`, it emitted a `DirectCall` to that top-level function.
- Crucially, if the identifier was in `nested_methods`, `bare_name_call` explicitly returned `None` to avoid emitting an invalid `DirectCall` to a top-level function, leaving the identifier to fall back to a local variable read (`Scope::Local`). Because no local variable with that name was ever declared, the SIR validator rejected the module with:
  ```text
  ValidatorIssue { message: "var-ref scope=local references unknown name 'm'", severity: Error }
  ```
Additionally, calls with explicit arguments or empty parentheses without a receiver (e.g. `m()`, `m(1)`) in `lower_method_call` fell back to `Expr::DirectCall { fn_name: callee, .. }`. Because class instance methods are hoisted to top-level functions with qualified names (`C__m`), `DirectCall` to `callee` referenced an undeclared top-level function and failed validation.

## Specification

1. **Per-Class Instance Method and Ancestry Discovery**:
   - `collect_method_names` collects:
     - `top_level_methods`: `HashSet<String>`
     - `nested_methods`: `HashSet<String>`
     - `class_instance_methods`: `HashMap<String, HashSet<String>>` (instance methods defined in class `C`, including `attr_reader`, `attr_writer`, and `attr_accessor` declarations)
     - `class_superclasses`: `HashMap<String, String>` (the declared superclass `< Superclass` for each class)
     - `class_includes`: `HashMap<String, Vec<String>>` (modules included via `include M` in each class)
     - `module_instance_methods`: `HashMap<String, HashSet<String>>` (instance methods defined in modules)
   - `Lowerer::class_has_instance_method(&self, class: &str, method: &str) -> bool` checks whether `method` is defined in `class`, any of its included modules, or any ancestor in its superclass hierarchy.

2. **Bare Identifier Resolution on `self`**:
   - In `Lowerer::bare_name_call(&mut self, name: &str, span: &Span) -> Option<Expr>`:
     - First, check if `name` is shadowed in local scope (`declared_locals`, `current_params`, `enclosing_visible`). If shadowed, return `None` (it is a local/parameter reference).
     - Second, if inside an instance method of a class (`self.current_class` is `Some(class_name)` and `self.current_method` is `Some(...)`) and `self.class_has_instance_method(class_name, name)`:
       - Lower as a dynamic method dispatch on `self`:
         ```rust
         self.features_used.insert(Feature::Classes);
         self.features_used.insert(Feature::Strings);
         Expr::BuiltinCall {
             name: "__method__".to_string(),
             args: vec![
                 Expr::BuiltinCall {
                     name: "__self__".to_string(),
                     args: Vec::new(),
                     effects: EffectSet::PURE,
                     span: span.clone(),
                 },
                 Expr::StrLit {
                     value: name.to_string(),
                     span: span.clone(),
                 },
             ],
             effects: EffectSet::PURE,
             span: span.clone(),
         }
         ```
     - Third, if `self.top_level_methods.contains(name)` and `!self.nested_methods.contains(name)`:
       - Lower as `Expr::DirectCall` to the top-level function.
     - Otherwise, return `None`.

3. **Method Calls with Parentheses or Arguments on `self`**:
   - In `Lowerer::lower_method_call`:
     - If `callee` is a recognized builtin (`ruby_builtin_effects`), lower via `builtin_call_or_sys_write`.
     - Else, if inside a class instance method (`self.current_class` is `Some(class_name)` and `self.current_method` is `Some(...)`) and `self.class_has_instance_method(class_name, &callee)`:
       - Lower as `Expr::BuiltinCall` with `name: "__method__"`, passing `__self__()`, `StrLit(callee)`, and the evaluated `args`.
       - Mark `Feature::Classes` and `Feature::Strings`.
     - Otherwise, fall back to `Expr::DirectCall { fn_name: callee, args, .. }`.

4. **Preservation of Method Tracking Context**:
   - Ensure `self.current_method` is consistently tracked around `lower_endless_def_statement` as well as `lower_def_statement`.
