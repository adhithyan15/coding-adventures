; expect: unsat
(set-logic QF_BV)
(declare-const a (_ BitVec 16))
(declare-const b (_ BitVec 16))
(assert (not (= (bvadd a b) (bvadd b a))))
(check-sat)
