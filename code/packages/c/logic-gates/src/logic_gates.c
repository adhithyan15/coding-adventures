/*
 * logic_gates.c — implementation of the eleven gates.  See logic_gates.h for
 * the contract (status-code errors, no pointers).
 * ===========================================================================
 *
 * VALIDATION
 * ----------
 * A bit is an int equal to 0 or 1.  The macro below is true exactly then.
 * Every public function checks ALL of its arguments first, so an invalid
 * input is reported no matter which position it is in or what the other
 * argument is.  (The NAND-built gates check too, rather than trusting the
 * lg_nand they call, so their contract is visible in their own body.)
 *
 * ARITHMETIC
 * ----------
 * Once inputs are known to be 0 or 1, `a & b`, `a | b`, `a ^ b` and `1 - a`
 * stay in {0, 1}.  No shifts and nothing that can overflow, so there is no
 * undefined behaviour anywhere in this file.
 */
#include "logic_gates.h"

#define LG_IS_BIT(x) ((x) == 0 || (x) == 1)

/* NOT: flip the bit.      a | out
 *                         0 |  1
 *                         1 |  0   */
int lg_not(int a) {
    if (!LG_IS_BIT(a)) {
        return LG_INVALID;
    }
    return 1 - a;
}

/* AND: 1 only when both are 1. */
int lg_and(int a, int b) {
    if (!LG_IS_BIT(a) || !LG_IS_BIT(b)) {
        return LG_INVALID;
    }
    return a & b;
}

/* OR: 1 when at least one is 1. */
int lg_or(int a, int b) {
    if (!LG_IS_BIT(a) || !LG_IS_BIT(b)) {
        return LG_INVALID;
    }
    return a | b;
}

/* XOR: 1 when the inputs differ. */
int lg_xor(int a, int b) {
    if (!LG_IS_BIT(a) || !LG_IS_BIT(b)) {
        return LG_INVALID;
    }
    return a ^ b;
}

/* NAND: NOT(AND).  The universal gate: 0 only when both are 1. */
int lg_nand(int a, int b) {
    if (!LG_IS_BIT(a) || !LG_IS_BIT(b)) {
        return LG_INVALID;
    }
    return 1 - (a & b);
}

/* NOR: NOT(OR).  1 only when both are 0. */
int lg_nor(int a, int b) {
    if (!LG_IS_BIT(a) || !LG_IS_BIT(b)) {
        return LG_INVALID;
    }
    return 1 - (a | b);
}

/* XNOR: NOT(XOR).  1 when the inputs are equal. */
int lg_xnor(int a, int b) {
    if (!LG_IS_BIT(a) || !LG_IS_BIT(b)) {
        return LG_INVALID;
    }
    return 1 - (a ^ b);
}

/* ---- Built from NAND only ---------------------------------------------- */

/* NOT from one NAND: tie both inputs together.   NAND(a,a) = NOT a
 *
 *     a ──┬──┐
 *         └──┤NAND├── out
 */
int lg_nand_not(int a) {
    if (!LG_IS_BIT(a)) {
        return LG_INVALID;
    }
    return lg_nand(a, a);
}

/* AND from two NANDs: NAND then invert.   AND(a,b) = NOT(NAND(a,b))
 *
 *     a ─┐
 *        ├NAND── n ──(tie both)──NAND── out
 *     b ─┘
 */
int lg_nand_and(int a, int b) {
    int n;
    if (!LG_IS_BIT(a) || !LG_IS_BIT(b)) {
        return LG_INVALID;
    }
    n = lg_nand(a, b);
    return lg_nand(n, n);
}

/* OR from three NANDs (De Morgan): OR(a,b) = NAND(NOT a, NOT b)
 *
 *     a ──(tie)──NAND── na ─┐
 *                           ├NAND── out
 *     b ──(tie)──NAND── nb ─┘
 */
int lg_nand_or(int a, int b) {
    int na, nb;
    if (!LG_IS_BIT(a) || !LG_IS_BIT(b)) {
        return LG_INVALID;
    }
    na = lg_nand(a, a);
    nb = lg_nand(b, b);
    return lg_nand(na, nb);
}

/* XOR from four NANDs, the classic construction:
 *
 *     n   = NAND(a, b)
 *     out = NAND( NAND(a, n), NAND(b, n) )
 *
 *     a ─┬─────────────┐
 *        │   NAND(a,n)=x ─┐
 *        └─n──┤            ├─NAND── out
 *     b ──────┤ NAND(b,n)=y ─┘
 *
 * Check a=1,b=0: n=1; x=NAND(1,1)=0; y=NAND(0,1)=1; out=NAND(0,1)=1.  XOR. */
int lg_nand_xor(int a, int b) {
    int n, x, y;
    if (!LG_IS_BIT(a) || !LG_IS_BIT(b)) {
        return LG_INVALID;
    }
    n = lg_nand(a, b);
    x = lg_nand(a, n);
    y = lg_nand(b, n);
    return lg_nand(x, y);
}
