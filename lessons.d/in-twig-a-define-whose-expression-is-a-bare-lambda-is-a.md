---
category: Compiler / VM / language pipeline
---

# In Twig a define whose expression is a bare lambda is a function define, so a test of a value define must wrap the lambda

While porting builtin-shadowing tests to `twig-to-semantic-ir` (#18456), a
test meant to exercise a *value* define named like a builtin used
`(define print (lambda (x) x))`. Both Twig frontends classify a define by the
shape of its expression: a bare `lambda` becomes a top-level **function**
(`DirectCall` in call position), and only a non-lambda expression becomes a
global **value** (`IndirectCall` on the `Global`). So the test asserted the
wrong call node and failed against the correct fix.

Fix: write value defines as `(define print (let ((k 0)) (lambda (x) k)))`, the
shape `twig-ir-compiler`'s `BUILTIN_SHADOWING_PROGRAMS` already uses, and pin
the bare-lambda spelling separately as a function define.

Related trap from the same change: `lower_apply` lowers the arguments before
the callee, so in `((lambda (a) …) (lambda (b) …))` the *argument* lambda is
`__lambda_0`. Find a synthesised lambda by its parameters, not its gensym.
