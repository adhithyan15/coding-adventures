/*
 * logic_gates.h — the seven basic logic gates, plus NAND-built versions, in
 * pure ISO C17.
 * ===========================================================================
 *
 * A logic gate is a tiny function from bits to a bit.  A bit is an `int` that
 * is exactly 0 or 1.  This package has eleven such functions and nothing else.
 *
 * THE ERROR MODEL: A STATUS CODE
 * ------------------------------
 * C has no exceptions.  So when an argument is not a bit (2, -7, INT_MAX ...)
 * every function returns LG_INVALID (-1).  -1 can never be a real gate output
 * (those are 0 or 1), so the caller can always tell the two apart:
 *
 *     int r = lg_and(x, y);
 *     if (r == LG_INVALID) { ... x or y was not 0/1 ... }
 *
 * WHY NO POINTERS
 * ---------------
 * Every function takes ints by value and returns an int.  No pointers, arrays,
 * allocation, I/O, globals or shifts.  That rules out, by construction, the
 * whole memory-safety family of bugs (out-of-bounds, use-after-free, null
 * dereference, leaks): there is simply no memory to get wrong.
 *
 * TRUTH TABLES (a b -> out)
 * -------------------------
 *    a b | AND OR XOR NAND NOR XNOR      a | NOT
 *    0 0 |  0   0   0    1    1    1     0 |  1
 *    0 1 |  0   1   1    1    0    0     1 |  0
 *    1 0 |  0   1   1    1    0    0
 *    1 1 |  1   1   0    0    0    1
 *
 * The lg_nand_* functions compute NOT/AND/OR/XOR using ONLY lg_nand, to show
 * that NAND alone is enough to build every other gate.
 *
 * Portability: GCC, Clang and MSVC with -pedantic-errors / /permissive- and
 * warnings-as-errors.  No extensions.
 */
#ifndef LOGIC_GATES_H
#define LOGIC_GATES_H

/* Returned by every function when ANY argument is not exactly 0 or 1. */
#define LG_INVALID (-1)

/* Direct gates. */
int lg_not(int a);
int lg_and(int a, int b);
int lg_or(int a, int b);
int lg_xor(int a, int b);
int lg_nand(int a, int b);
int lg_nor(int a, int b);
int lg_xnor(int a, int b);

/* Gates built only from lg_nand. */
int lg_nand_not(int a);
int lg_nand_and(int a, int b);
int lg_nand_or(int a, int b);
int lg_nand_xor(int a, int b);

#endif /* LOGIC_GATES_H */
