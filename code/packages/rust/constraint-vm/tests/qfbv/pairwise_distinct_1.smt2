; expect: unsat
; Three pairwise-different 1-bit values: pigeonhole.
(set-logic QF_BV)
(declare-const a (_ BitVec 1))
(declare-const b (_ BitVec 1))
(declare-const c (_ BitVec 1))
(assert (distinct a b))
(assert (distinct b c))
(assert (distinct a c))
(check-sat)
