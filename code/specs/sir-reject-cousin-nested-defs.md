# SIR Spec: Lexical Visibility for Nested Functions in Python Frontend

**Issue:** #18482 Item 4  
**Status:** Approved  
**Affected Crates:** `python-to-semantic-ir`

## Background

In Python, function definitions obey standard lexical scoping rules (LEGB: Local, Enclosing, Global, Builtin):
- A top-level `def` is bound in the module's global namespace and is visible from everywhere in the module.
- A nested `def` statement inside an enclosing function `P` binds its name in `P`'s local scope. It is visible to:
  1. `P` itself (the defining scope).
  2. Sibling functions defined directly inside `P` (enabling mutual recursion between nested functions).
  3. Functions nested within `P` or its descendants (inner scopes).
  4. The function itself (self-recursion).
- A nested `def` is NOT visible to:
  1. Unrelated top-level statements or the module body outside `P`.
  2. Other top-level functions or functions defined under other parent functions ("cousins").

Prior to this specification, `python-to-semantic-ir` collected all `def` statements across the entire module into a single flat `function_names` set during Pass 1. During Pass 2 lowering:
- Any call or bare reference to any function name in `function_names` resolved to a `DirectCall` or `MakeClosure`, regardless of the caller's lexical context.
- Consequently, cousin functions and top-level code could freely call or reference nested functions from unrelated scopes, whereas Python raises `NameError`.

## Specification

1. **Context Hierarchy**:
   Every `FunctionCtx` tracks its immediate defining parent `parent_owner: Option<String>` in addition to its `owner: String`.
   - Top-level `main` context has `owner: MODULE_OWNER` and `parent_owner: None`.
   - Method contexts have `owner: DETACHED_OWNER` and `parent_owner: None`.
   - Nested `def` and `lambda` contexts inherit `parent_owner: Some(enclosing.owner.clone())`.

2. **Function Visibility Check**:
   Before resolving a function name `target` to a direct call or closure reference:
   - If `target` is a top-level function (`def_facts.get(target).parent == MODULE_OWNER`), it is visible everywhere.
   - Otherwise, `target` is a nested function defined in scope $S = \text{def\_facts}(target).\text{parent}$. It is visible from context `ctx` if and only if $S$ is an ancestor of `ctx.owner` in the lexical scope hierarchy (including `ctx.owner == S`).

3. **Error Reporting on Inaccessible Names**:
   If a call or bare reference names a function that exists in the module but is not lexically visible in the caller's scope, the lowerer MUST NOT resolve it as a known function. It falls through to standard name resolution, producing an `"unresolved name <name>"` error (Python `NameError`), with the exact line and column of the expression.
