; expect: sat
; x + 1 = 5 and x <u 15: x = 4.
(set-logic QF_BV)
(declare-const x (_ BitVec 8))
(assert (= (bvadd x #x01) (_ bv5 8)))
(assert (bvult x #b00001111))
(check-sat)
(get-model)
