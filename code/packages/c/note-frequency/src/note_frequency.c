/*
 * note_frequency.c — implementation of the note-name -> frequency library.
 * ===========================================================================
 * No <math.h>: the one power 2^x is 2^x = e^(x*ln2), with e^x computed by
 * Cody-Waite range reduction + a Taylor series (see d_exp).
 */
#include "note_frequency.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define NF_REFERENCE_OCTAVE 4
#define NF_REFERENCE_INDEX 9
#define NF_REFERENCE_FREQUENCY_HZ 440.0
#define NF_SEMITONES_PER_OCTAVE 12

/* ---------------------------------------------------------------------------
 *  <math.h>-free exp / exp2
 * ------------------------------------------------------------------------- */

static double pow2i(int k) {
    double result = 1.0;
    double base = k < 0 ? 0.5 : 2.0;
    int n = k < 0 ? -k : k;
    while (n > 0) {
        if (n & 1) {
            result *= base;
        }
        base *= base;
        n >>= 1;
    }
    return result;
}

/* e^x via Cody-Waite range reduction (x = k*ln2 + r, |r| <= ln2/2). */
static double d_exp(double x) {
    if (x != x) {
        return x; /* NaN propagates (and stays out of the int cast) */
    }
    if (x == 0.0) {
        return 1.0;
    }
    /* Bound |x| before the (int) range reduction: an extreme octave can drive
     * the exponent past these limits, and casting a huge double to int is UB.
     * e^x overflows above ~709.78 and underflows below ~-745.13 anyway. */
    if (x > 709.782712893384) {
        return 1.7976931348623157e308;
    }
    if (x < -745.13321910194) {
        return 0.0;
    }
    const double INV_LN2 = 1.4426950408889634;
    const double C1 = 0.693359375; /* exact; C1 + C2 == ln2 */
    const double C2 = -2.1219444005469058277e-4;
    double kf = x * INV_LN2;
    int k = (int)(kf >= 0.0 ? kf + 0.5 : kf - 0.5);
    double r = (x - (double)k * C1) - (double)k * C2;
    double term = 1.0;
    double sum = 1.0;
    int i;
    for (i = 1; i <= 17; i++) {
        term *= r / (double)i;
        sum += term;
    }
    return sum * pow2i(k);
}

/* 2^y == e^(y*ln2). Our y values are small (a few octaves), so no overflow. */
static double exp2_d(double y) {
    const double LN2 = 0.6931471805599453;
    return d_exp(y * LN2);
}

/* ---------------------------------------------------------------------------
 *  Chromatic spelling table
 * ------------------------------------------------------------------------- */

/* Return the 0..11 chromatic index for a spelling, or -1 if unsupported. Only
 * the spellings that name a real pitch are listed (so "Cb"/"E#"/... map to -1). */
static int chromatic_index_for(const char *spelling) {
    struct {
        const char *s;
        int idx;
    } table[] = {
        {"C", 0},  {"C#", 1},  {"Db", 1}, {"D", 2},  {"D#", 3}, {"Eb", 3},
        {"E", 4},  {"F", 5},   {"F#", 6}, {"Gb", 6}, {"G", 7},  {"G#", 8},
        {"Ab", 8}, {"A", 9},   {"A#", 10}, {"Bb", 10}, {"B", 11},
    };
    size_t i;
    for (i = 0; i < sizeof table / sizeof table[0]; i++) {
        if (strcmp(spelling, table[i].s) == 0) {
            return table[i].idx;
        }
    }
    return -1;
}

static char upper_ascii(char c) {
    if (c >= 'a' && c <= 'z') {
        return (char)(c - 'a' + 'A');
    }
    return c;
}

/* ---------------------------------------------------------------------------
 *  Note
 * ------------------------------------------------------------------------- */

NfStatus nf_note_new(const char *letter, const char *accidental, int octave,
                     NfNote *out) {
    /* Canonicalize the letter to uppercase; build "letter+accidental". */
    char canonical = upper_ascii(letter[0]);
    char spelling[8];
    snprintf(spelling, sizeof spelling, "%c%s", canonical, accidental);
    if (chromatic_index_for(spelling) < 0) {
        return NF_ERR_INVALID_SPELLING;
    }
    out->letter = canonical;
    /* accidental is "", "#", or "b" (already validated by the lookup). */
    size_t alen = strlen(accidental);
    if (alen >= sizeof out->accidental) {
        return NF_ERR_INVALID_SPELLING; /* defensive; never hit for #/b/"" */
    }
    memcpy(out->accidental, accidental, alen + 1);
    out->octave = octave;
    return NF_OK;
}

void nf_note_spelling(const NfNote *n, char *buf, size_t bufsz) {
    snprintf(buf, bufsz, "%c%s", n->letter, n->accidental);
}

int nf_note_chromatic_index(const NfNote *n) {
    char spelling[4];
    snprintf(spelling, sizeof spelling, "%c%s", n->letter, n->accidental);
    return chromatic_index_for(spelling);
}

/* Semitone distance in a WIDE type: octave can be any int (parsing accepts the
 * full i32 range like the Rust crate), so `(octave - 4) * 12` is done in
 * long long to avoid signed-overflow UB for extreme octaves. */
static long long semitones_ll(const NfNote *n) {
    long long octave_offset = ((long long)n->octave - NF_REFERENCE_OCTAVE) *
                              NF_SEMITONES_PER_OCTAVE;
    long long pitch_offset =
        (long long)nf_note_chromatic_index(n) - NF_REFERENCE_INDEX;
    return octave_offset + pitch_offset;
}

int nf_note_semitones_from_a4(const NfNote *n) {
    /* For any musically sensible octave this is exact; the narrowing cast for a
     * pathological octave is implementation-defined, never undefined. */
    return (int)semitones_ll(n);
}

double nf_note_frequency(const NfNote *n) {
    /* Use the wide value so the frequency is well-defined even for extreme
     * octaves (which simply saturate to 0 or +inf). */
    double exponent = (double)semitones_ll(n) / (double)NF_SEMITONES_PER_OCTAVE;
    return NF_REFERENCE_FREQUENCY_HZ * exp2_d(exponent);
}

void nf_note_to_string(const NfNote *n, char *buf, size_t bufsz) {
    snprintf(buf, bufsz, "%c%s%d", n->letter, n->accidental, n->octave);
}

/* ---------------------------------------------------------------------------
 *  Parsing
 * ------------------------------------------------------------------------- */

/* True if text is an optional leading '-' followed by one or more digits. */
static int is_canonical_octave(const char *text) {
    const char *p = text;
    if (*p == '-') {
        p++;
    }
    if (*p == '\0') {
        return 0; /* empty (or just "-") */
    }
    for (; *p != '\0'; p++) {
        if (*p < '0' || *p > '9') {
            return 0;
        }
    }
    return 1;
}

/* Parse a canonical octave string to int (assumes is_canonical_octave passed).
 * Returns 0 on success, -1 on overflow/format error. */
static int parse_int(const char *text, int *out) {
    long v;
    char *end;
    v = strtol(text, &end, 10);
    if (*end != '\0') {
        return -1;
    }
    if (v < -2147483647L - 1 || v > 2147483647L) {
        return -1;
    }
    *out = (int)v;
    return 0;
}

NfStatus nf_parse_note(const char *text, NfNote *out) {
    if (text == NULL || text[0] == '\0') {
        return NF_ERR_INVALID_NOTE;
    }
    char letter = text[0];
    char up = upper_ascii(letter);
    if (up < 'A' || up > 'G') {
        return NF_ERR_INVALID_NOTE;
    }

    const char *rest = text + 1;
    const char *accidental = "";
    const char *octave_text;
    if (rest[0] == '#') {
        accidental = "#";
        octave_text = rest + 1;
    } else if (rest[0] == 'b') {
        accidental = "b";
        octave_text = rest + 1;
    } else {
        octave_text = rest;
    }

    if (octave_text[0] == '\0' || !is_canonical_octave(octave_text)) {
        return NF_ERR_INVALID_NOTE;
    }
    int octave;
    if (parse_int(octave_text, &octave) != 0) {
        return NF_ERR_INVALID_NOTE;
    }

    char letter_str[2];
    letter_str[0] = letter;
    letter_str[1] = '\0';
    return nf_note_new(letter_str, accidental, octave, out);
}

NfStatus nf_note_to_frequency(const char *text, double *out) {
    NfNote n;
    NfStatus st = nf_parse_note(text, &n);
    if (st != NF_OK) {
        return st;
    }
    *out = nf_note_frequency(&n);
    return NF_OK;
}
