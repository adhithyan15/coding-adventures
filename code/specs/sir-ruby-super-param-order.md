# SIR Spec: Parameter Declaration Order for Zero-Argument Super in Ruby

**Issue:** #18798 Item 20  
**Status:** Approved  
**Affected Crates:** `ruby-to-semantic-ir`

## Background

In Ruby, the zero-argument `super` keyword (often termed `zsuper`) re-dispatches the call to the superclass method, forwarding all arguments of the enclosing method by reference:
```ruby
class Base
  def greet(greeting, target)
    # ...
  end
end

class Derived < Base
  def greet(greeting, target)
    super
  end
end
```

Crucially, zero-argument `super` must pass the parameters in their exact source declaration order:
`def m(b, a); super; end` must forward `(b, a)` to the superclass method.

Prior to this specification, `ruby-to-semantic-ir/src/lower.rs` tracked parameters only in `current_params: HashSet<String>`. When lowering zero-argument `super`:
```rust
let mut params: Vec<String> = self.current_params.iter().cloned().collect();
params.sort();
```
Because `HashSet` has no deterministic iteration order, the lowerer sorted `params` alphabetically. Consequently, any method whose parameters were not in alphabetical order (e.g. `def m(b, a)`, `def write(buf, offset, count)`) had its arguments scrambled or reversed when forwarded via `super`.

Additionally, entering blocks or lambdas inside a method scoped `current_params` to the block's parameters, preventing nested blocks from forwarding the enclosing method's arguments.

## Specification

1. **Method Parameter Declaration Order Tracking**:
   `RubyLowerer` maintains `current_method_params: Vec<String>` containing the parameter names of the enclosing method in their exact declaration order from the AST.

2. **Method Scope Entry & Exit**:
   - In `lower_def_statement` and `lower_endless_def_statement`, `self.current_method_params` and `self.current_method` are saved prior to lowering the body and restored upon completion.
   - `self.current_method_params` is initialized to `params.iter().map(|p| p.name.clone()).collect()`.
   - Block and lambda lowering (`lower_block_literal`, `lower_lambda_expr`) MUST NOT overwrite `self.current_method_params`, allowing zero-argument `super` within a block to correctly reference the enclosing method's parameters.

3. **Zero-Argument Super Lowering**:
   - In `lower_super_expr`, when no explicit arguments are provided (`super_args` is `None`), the forwarded arguments are constructed directly by iterating over `self.current_method_params`.
   - Each parameter is mapped to `Expr::VarRef { name: p, scope: Scope::Param, span: self.span_of(node) }` in order.
   - Alphabetical sorting is eliminated.
