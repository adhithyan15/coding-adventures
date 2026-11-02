; expect: unsat
(set-logic QF_BV)
(declare-const a (_ BitVec 32))
(declare-const b (_ BitVec 32))
(assert (distinct (bvxor (bvxor a b) b) a))
(check-sat)
