; expect: sat
; |x| as (x <s 0 ? -x : x) is negative for x = -32768: a real counterexample.
(set-logic QF_BV)
(declare-const x (_ BitVec 16))
(assert (bvslt (ite (bvslt x #x0000) (bvneg x) x) #x0000))
(check-sat)
