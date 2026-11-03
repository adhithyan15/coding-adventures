; expect: unsat
; a - b = a + (-b) for every pair of 32-bit integers.
(set-logic QF_BV)
(declare-const a (_ BitVec 32))
(declare-const b (_ BitVec 32))
(assert (distinct (bvsub a b) (bvadd a (bvneg b))))
(check-sat)
