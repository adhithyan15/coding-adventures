---
category: Cryptography & security review
---

# A shared certificate test policy must cover every fixture key size before a composition test can reach its intended boundary

A construction-bound reviewed-path test added a real 2048-bit Let's Encrypt
R13 intermediate beside the crate's existing 4096-bit ISRG Root X1 fixture.
The shared algorithm-profile helper required a 4096-bit encoded subject key,
so every new test failed in algorithm policy before exercising the evidence
binder. Before composing certificate fixtures, compare each fixture's key size
with every shared algorithm and RSA policy bound. For mixed public chains, use
the narrow range the fixtures actually require, while leaving production policy
caller-owned and data-driven.
