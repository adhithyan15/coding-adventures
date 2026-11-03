; expect: sat unsat
; Negative as signed, yet above 100 as unsigned (e.g. #x80): sat.
; Negative as signed and below 128 as unsigned: impossible.
(set-logic QF_BV)
(declare-const x (_ BitVec 8))
(push 1)
(assert (bvslt x #x00))
(assert (bvugt x (_ bv100 8)))
(check-sat)
(pop 1)
(assert (bvslt x #x00))
(assert (bvult x #x80))
(check-sat)
