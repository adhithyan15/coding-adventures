; expect: sat
; 3 is odd, hence invertible mod 2^16 (the inverse is 43691).
(set-logic QF_BV)
(declare-const x (_ BitVec 16))
(assert (= (bvmul x (_ bv3 16)) (_ bv1 16)))
(check-sat)
