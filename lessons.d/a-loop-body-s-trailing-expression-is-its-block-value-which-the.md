---
category: Compiler / VM / language pipeline
---

# A loop body's trailing expression is its block value, which the C and Ruby backends never emit, so a frontend must lower loop bodies as statements only

`python-to-semantic-ir` lowered every suite with `assemble_block`, which
makes a trailing expression statement the block's *value*.  For a `while`
or `for` body that value is meaningless, and the C and Ruby backends emit
only a loop body's statements.  So `for i in range(3): print(i)` printed
nothing on C and Ruby, and neither did a trailing `if` or call, while the
other four backends printed correctly (#18482 review).  The first round of
end-to-end tests missed it because no program printed from inside a loop.

What to do: a frontend should give a loop body no value (move a trailing
expression into an `ExprStmt`), and an end-to-end corpus should include a
loop whose *last* statement is a `print` or an `if`.  More generally, when
a construct's value is discarded by the IR's semantics, do not rely on
every backend evaluating it anyway.
