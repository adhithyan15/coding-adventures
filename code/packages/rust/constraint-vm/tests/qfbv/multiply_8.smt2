; expect: sat unsat
; A factorisation of 143 mod 256 with both factors above 1: sat.
; Doubling never yields an odd number: unsat.
(set-logic QF_BV)
(declare-const x (_ BitVec 8))
(declare-const y (_ BitVec 8))
(push 1)
(assert (= (bvmul x y) (_ bv143 8)))
(assert (bvugt x #x01))
(assert (bvugt y #x01))
(check-sat)
(pop 1)
(assert (= (bvmul x #x02) #x01))
(check-sat)
