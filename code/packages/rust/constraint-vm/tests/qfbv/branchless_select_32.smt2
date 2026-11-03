; expect: unsat
; The constant-time select used by ct-compare style code:
;   mask = -(c ? 1 : 0);  (mask & a) | (~mask & b)  ==  c ? a : b
(set-logic QF_BV)
(declare-const c Bool)
(declare-const a (_ BitVec 32))
(declare-const b (_ BitVec 32))
(assert (distinct
  (bvor (bvand (bvneg (ite c (_ bv1 32) (_ bv0 32))) a)
        (bvand (bvnot (bvneg (ite c (_ bv1 32) (_ bv0 32)))) b))
  (ite c a b)))
(check-sat)
