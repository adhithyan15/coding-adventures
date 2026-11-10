---
category: Testing & coverage
---

# A sabotage applied to every operation of a field hides from every relational claim

The first run of the generated gf256 evidence failed its own negative controls:
`negctl/.../divide/alt-poly` ("compute `divide` in the AES field, polynomial
0x11B, instead of 0x11D") was accepted by every suite of `divide`. The fault
enum is global, and `multiply`, `divide`, `inverse` and `power` all listed the
`alt-poly` fault, so the claim `divide(multiply(a, b), b) == Ok(a)` ran with
*both* operations in the AES field. The AES field is a perfectly good field, so
`divide` still undid `multiply`, and every relational claim (associativity,
distributivity, inverse, divide/multiply) passed. Only the claim that compares
`multiply` with an independent shift-and-add model can tell the two polynomials
apart, and it is attributed to `multiply`, not `divide`.

**Do instead:**

* inject a fault into the one operation under attack and keep every other
  adapter honest (the generator now scopes a scalar fault with a thread-local
  "attacked operation", `attacking(op, ..)` / `fault_for(op, fault)`); a fault
  applied consistently to a whole algebraic structure is a different, valid
  structure and relational claims will correctly pass it;
* never trust field-law claims to pin a specific field: every one of them holds
  in all 16 primitive-polynomial fields. Pin the polynomial with an independent
  oracle written in the specification language (here `gmul`, a Horner
  shift-and-add model, plus a bit-level XOR model) and with known vectors that
  differ between the candidate polynomials;
* stop a negative control at the first claim that catches the fault: a control
  that runs every claim of the operation to completion re-ran a 2^24-case law
  that cannot catch the fault, and tripled the wall-clock cost. Order claims so
  the model-pinned (cheap, strong) claim comes first.
