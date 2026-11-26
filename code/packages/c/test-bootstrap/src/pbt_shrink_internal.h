/*
 * pbt_shrink_internal.h -- the shrinkers shared by pbt_gen.c and pbt_shrink.c.
 */
#ifndef PBT_SHRINK_INTERNAL_H
#define PBT_SHRINK_INTERNAL_H

#include "pbt.h"

/* Candidate test for the integer walk: 1 while the candidate STILL FAILS. */
typedef int (*pbt_big_falsifies_fn)(void *self, const pbt_big *candidate);

/*
 * Bisect then unit-step *current toward target.  lo / hi may be NULL
 * (unbounded).  *steps receives the accepted moves; *current is updated in place.
 */
void pbt_int_walk(pbt_big *current, const pbt_big *target, const pbt_big *lo, const pbt_big *hi,
                  pbt_big_falsifies_fn falsifies, void *self, int budget, int *steps);

/* The target probe (one step) followed by the walk.  max_steps == 0 does nothing. */
void pbt_int_shrink_core(pbt_big *current, const pbt_big *target, const pbt_big *lo, const pbt_big *hi,
                         pbt_big_falsifies_fn falsifies, void *self, int max_steps, int *steps);

/*
 * Shrink a falsifying BYTES or LIST value in place (VER05 section 4.3):
 *   probe the length-min_len prefix, bisect, delete one element at a time right
 *   to left, then shrink each element.
 *
 * Element shrinking: `elem` (a generator's shrinker) when non-NULL, else the
 * byte walk for BYTES, else an unbounded integer walk toward `int_target` for a
 * LIST of INT when it is non-NULL, else none.
 */
int pbt_shrink_sequence(pbt_budget *b, pbt_val *seq, pbt_pred falsifies, int max_steps, size_t min_len,
                        const pbt_gen *elem, const pbt_big *int_target, int *steps);

#endif /* PBT_SHRINK_INTERNAL_H */
