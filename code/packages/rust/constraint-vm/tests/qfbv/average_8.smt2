; expect: unsat
; The overflow-free average (a & b) + ((a ^ b) >> 1) equals (a + b) >> 1
; whenever a + b cannot overflow.
(set-logic QF_BV)
(declare-const a (_ BitVec 8))
(declare-const b (_ BitVec 8))
(assert (bvule a #x7f))
(assert (bvule b #x7f))
(assert (distinct (bvadd (bvand a b) (bvlshr (bvxor a b) #x01))
                  (bvlshr (bvadd a b) #x01)))
(check-sat)
