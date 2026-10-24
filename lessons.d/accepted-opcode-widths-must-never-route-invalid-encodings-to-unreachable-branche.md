---
category: Cryptography & security review
---

# Accepted opcode widths must never route invalid encodings to unreachable branches

The System/360 width classifier deliberately accepted a contiguous SI opcode
range, but that range contains an unused `0x93` encoding. The SI executor then
treated its default arm as unreachable, so untrusted program bytes could panic
after the program counter changed instead of returning a typed atomic fault.

Every decoder reached by untrusted bytes must retain a fallible default even
when an earlier classifier appears exhaustive. Add a regression for each hole
in a dense opcode range, assert the exact typed error, and assert that the
complete machine state is unchanged.
