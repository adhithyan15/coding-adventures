# SIR Spec: Refuse Splat Call Arguments in Python Frontend

**Issue:** #18482 Item 2  
**Status:** Approved  
**Affected Crates:** `python-to-semantic-ir`

## Background

In Python, calls may pass arguments with unpacking syntax:
- `*args` unpacks an iterable sequence into positional arguments (`f(*xs)`).
- `**kwargs` unpacks a dictionary/mapping into keyword arguments (`f(**d)`).

Prior to this specification, `python-to-semantic-ir` did not detect or reject the leading `*` (`STAR`) or `**` (`DOUBLE_STAR`) tokens on `argument` CST nodes. Instead, `single_arg_expr` stripped tokens and extracted the inner `expression` (`xs` or `d`), pushing it as a single positional argument.

Consequently:
- `f(*xs)` lowered to `f(xs)` (passing the sequence object as a single positional parameter).
- `f(**d)` lowered to `f(d)` (passing the dict object as a single positional parameter).
- When calling capturing functions, these arguments erroneously satisfied exact-arity checks, masking runtime bugs and creating silent divergence between CPython and SIR backends.

Parameters with `*args` and `**kwargs` are already refused:
- `"unsupported: *args positional-rest parameter (deferred)"`
- `"unsupported: **kwargs keyword-rest parameter (deferred)"`
- `"unsupported: default, *args, or **kwargs parameter in lambda (deferred)"`

## Specification

1. **Rejection in `lower_call_args`**:
   During argument lowering for indirect calls, direct calls, and method calls, each `argument` AST node is inspected for splat tokens before lowering:
   - If an `argument` contains a `*` (`STAR`) token preceding an expression (not followed by another `*`), it MUST be rejected with a positioned `PythonLowerError`:
     `"unsupported: *starred argument in call (deferred)"`
   - If an `argument` contains a `**` (`DOUBLE_STAR`) token preceding an expression, it MUST be rejected with a positioned `PythonLowerError`:
     `"unsupported: **double-starred keyword argument in call (deferred)"`

2. **Rejection in `try_range_for` and `class_base_name`**:
   Any argument node with a `*` or `**` token appearing in `range(...)` calls within `for` loop headers or class base expressions MUST similarly be rejected with a positioned error rather than stripping the splat operator.

3. **Line and Column Positioning**:
   The error line and column numbers MUST point to the `argument` node (or its leading splat token) where the unsupported syntax appeared.
