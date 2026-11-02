; expect: unsat unsat unsat sat
; SMT-LIB: shifting by the width or more gives zero (sign bits for bvashr).
(set-logic QF_BV)
(declare-const x (_ BitVec 8))
(push 1)
(assert (distinct (bvshl x (_ bv8 8)) #x00))
(check-sat)
(pop 1)
(push 1)
(assert (distinct (bvlshr x #x09) #x00))
(check-sat)
(pop 1)
(push 1)
(assert (and (bvslt x #x00) (distinct (bvashr x (_ bv200 8)) #xff)))
(check-sat)
(pop 1)
; Shifting by 7 can still leave something.
(assert (distinct (bvshl x #x07) #x00))
(check-sat)
