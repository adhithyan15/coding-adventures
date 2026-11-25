/*
 * pbt_tfv.h -- strict validator for TFV v1 text (VER05i section 8).
 *
 * TFV is the line format that carries test corpora between languages (TF01
 * section 2).  This is a validator, not a reader: its one job here is to let
 * the pbt-v1 `corpus_reject` cases be checked by a real parser.
 *
 *   tfv 1
 *   # comment
 *   <id> R<n> K<field> <value> K<field> <value> ...
 *
 * value := N | T | F | I<int> | D<16 hex> | X<hex> | S<hex> | L<n> v... | M<n> k v...
 *
 * Tokens are separated by exactly one space.  Rejected: uppercase or odd-length
 * hex, "I-0", "I007", a count that disagrees with the tokens, trailing tokens,
 * doubled spaces, a duplicate field / record id / map key, invalid UTF-8 inside
 * S, a carriage return in a record line, nesting deeper than 64 containers,
 * integers of more than 1000 digits, input over 1 MiB, more than 200,000 tokens.
 */
#ifndef PBT_TFV_H
#define PBT_TFV_H

#include <stddef.h>

#define PBT_TFV_MAX_DEPTH 64
#define PBT_TFV_MAX_INT_DIGITS 1000
#define PBT_TFV_MAX_BYTES ((size_t)1 << 20)
#define PBT_TFV_MAX_TOKENS 200000

/* 0 if text[0..len) is a well-formed TFV v1 document; otherwise nonzero and a short message in err. */
int pbt_tfv_validate(const char *text, size_t len, char *err, size_t errcap);

#endif /* PBT_TFV_H */
