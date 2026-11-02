; expect: unsat
(set-logic QF_BV)
(declare-const a (_ BitVec 8))
(declare-const b (_ BitVec 8))
(assert (distinct (bvnot (bvand a b)) (bvor (bvnot a) (bvnot b))))
(check-sat)
