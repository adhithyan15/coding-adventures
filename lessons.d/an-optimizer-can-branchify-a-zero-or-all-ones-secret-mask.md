---
category: Cryptography & security review
---

# An optimizer can branchify a zero or all-ones secret mask

The GHASH fixed-memory PR used a 0/255 input-bit mask instead of a source
branch. Rust 1.99 optimized the sixteen XOR updates into a conditional branch
that skipped them when the secret bit was zero. The optimized-IR test caught
two branches where only the public 128-round loop branch was allowed.

Keep that gate strict. The repair puts the completed mask behind a best-effort
optimizer barrier through a one-byte wipe-on-drop owner. Inspect the generated
IR again, including the new owner and its cleanup. `black_box` does not promise
constant-time cryptography; evidence is limited to each compiler/target tested.

Unwind lowering can add cleanup wipes and targets can choose different stack
alignment or declaration order. Check the bounded owner sizes and required
wipes without treating extra wipes or equivalent alignment as a failure.
