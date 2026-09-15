## 0.89.0 — 2026-08-11 — split formal specifications

Complementary report-style specification parts now merge before procedure
lowering: `integer a; array a;` forms an integer-array formal, and `integer p;
procedure p;` forms a typed procedure formal. Duplicate type/kind parts and
conflicting types or kinds still fail closed. The resulting array descriptors
and direct formal-procedure specialization retain their existing typed ABIs.

