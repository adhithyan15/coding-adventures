---
category: Rust
---

# A policy borrowing a rule slice needs the array bound before construction

An integration test constructed a borrowed `ExtensionContextPolicy` directly
from `&[rule]` and then used the policy in later statements. Rust correctly
rejected that with E0716 because the array literal was dropped at the end of
the constructor statement. Bind the array to a local variable before passing
its slice to any object that retains the borrow; an inline array is suitable
only when the borrower is consumed in the same statement.
