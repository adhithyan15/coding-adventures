/*
 * note_frequency.c — implementation of the note-name -> frequency library.
 * ===========================================================================
 * No <math.h>: the one power 2^x is 2^x = e^(x*ln2), with e^x computed by
 * Cody-Waite range reduction + a Taylor series (see d_exp).
 */
#include "note_frequency.h"

#include <float.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#define NF_REFERENCE_OCTAVE 4
#define NF_REFERENCE_INDEX 9
#define NF_REFERENCE_FREQUENCY_HZ 440.0
#define NF_SEMITONES_PER_OCTAVE 12

/* ---------------------------------------------------------------------------
 *  <math.h>-free finite check and exp / exp2
 * ------------------------------------------------------------------------- */

static int is_finite_dbl(double x) {
    return (x == x) && ((x - x) == 0.0) && (x <= DBL_MAX) && (x >= -DBL_MAX);
}

static double pow2i(int k) {
    double result = 1.0;
    double base = k < 0 ? 0.5 : 2.0;
    /* Convert magnitude in unsigned int so k == INT_MIN never triggers UB. */
    unsigned int n = k < 0 ? (unsigned int)(-(k + 1)) + 1u : (unsigned int)k;
    while (n > 0u) {
        if (n & 1u) {
            result *= base;
        }
        n >>= 1;
        if (n > 0u) {
            base *= base;
        }
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
    /* Bound |x| before the (int) range reduction: an extreme exponent can
     * overflow double/int; e^x overflows above ~709.78 and underflows below
     * ~-745.13 anyway. */
    if (x > 709.782712893384) {
        return DBL_MAX;
    }
    if (x < -745.13321910194) {
        return 0.0;
    }
    {
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
}

/* 2^y == e^(y*ln2). */
static double exp2_d(double y) {
    const double LN2 = 0.6931471805599453;
    return d_exp(y * LN2);
}

/* ---------------------------------------------------------------------------
 *  Chromatic spelling table and bounded string helpers
 * ------------------------------------------------------------------------- */

/* Return the 0..11 chromatic index for a spelling, or -1 if unsupported. Only
 * the spellings that name a real pitch are listed (so "Cb"/"E#"/... map to -1). */
static int chromatic_index_for(const char *spelling) {
    static const struct {
        const char *s;
        int idx;
    } table[] = {
        {"C", 0},  {"C#", 1},  {"Db", 1}, {"D", 2},  {"D#", 3}, {"Eb", 3},
        {"E", 4},  {"F", 5},   {"F#", 6}, {"Gb", 6}, {"G", 7},  {"G#", 8},
        {"Ab", 8}, {"A", 9},   {"A#", 10}, {"Bb", 10}, {"B", 11},
    };
    size_t i;
    if (spelling == NULL) {
        return -1;
    }
    for (i = 0; i < sizeof table / sizeof table[0]; i++) {
        if (strcmp(spelling, table[i].s) == 0) {
            return table[i].idx;
        }
    }
    return -1;
}

static int chromatic_index_parts(char letter, const char *accidental) {
    char spelling[4];
    if (accidental == NULL) {
        return -1;
    }
    if (accidental[0] == '\0') {
        spelling[0] = letter;
        spelling[1] = '\0';
    } else if (accidental[1] == '\0') {
        spelling[0] = letter;
        spelling[1] = accidental[0];
        spelling[2] = '\0';
    } else {
        return -1;
    }
    return chromatic_index_for(spelling);
}

static char upper_ascii(char c) {
    if (c >= 'a' && c <= 'z') {
        return (char)(c - 'a' + 'A');
    }
    return c;
}

/* Measure at most (max_len + 1) bytes so oversized hostile strings fail in O(1). */
static size_t bounded_strlen(const char *s, size_t max_len) {
    size_t len = 0;
    while (len <= max_len && s[len] != '\0') {
        len++;
    }
    return len;
}

/* ---------------------------------------------------------------------------
 *  Note
 * ------------------------------------------------------------------------- */

NfStatus nf_note_new(const char *letter, const char *accidental, int octave,
                     NfNote *out) {
    char canonical;
    if (letter == NULL || accidental == NULL || out == NULL) {
        return NF_ERR_NULL_POINTER;
    }
    /* Reject empty or multi-character letters (CWE-20, no silent truncation). */
    if (letter[0] == '\0' || letter[1] != '\0') {
        return NF_ERR_INVALID_SPELLING;
    }
    /* Reject oversized or multi-character accidentals in O(1). */
    if (accidental[0] != '\0' && accidental[1] != '\0') {
        return NF_ERR_INVALID_SPELLING;
    }
    if (octave < NF_MIN_OCTAVE || octave > NF_MAX_OCTAVE) {
        return NF_ERR_INVALID_NOTE;
    }

    canonical = upper_ascii(letter[0]);
    if (chromatic_index_parts(canonical, accidental) < 0) {
        return NF_ERR_INVALID_SPELLING;
    }

    out->letter = canonical;
    out->accidental[0] = accidental[0];
    out->accidental[1] = '\0';
    out->accidental[2] = '\0';
    out->octave = octave;
    return NF_OK;
}

void nf_note_spelling(const NfNote *n, char *buf, size_t bufsz) {
    size_t pos = 0;
    if (buf == NULL || bufsz == 0) {
        return;
    }
    if (nf_note_chromatic_index(n) < 0) {
        buf[0] = '\0';
        return;
    }
    if (pos + 1 < bufsz) {
        buf[pos++] = n->letter;
    }
    if (n->accidental[0] != '\0' && pos + 1 < bufsz) {
        buf[pos++] = n->accidental[0];
    }
    buf[pos] = '\0';
}

int nf_note_chromatic_index(const NfNote *n) {
    if (n == NULL) {
        return -1;
    }
    /* Guard against non-NUL-terminated accidental in caller-initialized structs. */
    if (n->accidental[0] != '\0' && n->accidental[1] != '\0') {
        return -1;
    }
    return chromatic_index_parts(n->letter, n->accidental);
}

/* Semitone distance in int64_t after validating octave bounds and spelling. */
static NfStatus checked_semitones_i64(const NfNote *n, int64_t *out) {
    int idx;
    int64_t octave_offset;
    int64_t pitch_offset;
    if (n == NULL || out == NULL) {
        return NF_ERR_NULL_POINTER;
    }
    idx = nf_note_chromatic_index(n);
    if (idx < 0) {
        return NF_ERR_INVALID_SPELLING;
    }
    if (n->octave < NF_MIN_OCTAVE || n->octave > NF_MAX_OCTAVE) {
        return NF_ERR_INVALID_NOTE;
    }
    octave_offset = ((int64_t)n->octave - (int64_t)NF_REFERENCE_OCTAVE) *
                    (int64_t)NF_SEMITONES_PER_OCTAVE;
    pitch_offset = (int64_t)idx - (int64_t)NF_REFERENCE_INDEX;
    *out = octave_offset + pitch_offset;
    return NF_OK;
}

NfStatus nf_note_checked_semitones_from_a4(const NfNote *n, int *out) {
    int64_t st64;
    NfStatus status;
    if (n == NULL || out == NULL) {
        return NF_ERR_NULL_POINTER;
    }
    status = checked_semitones_i64(n, &st64);
    if (status != NF_OK) {
        return status;
    }
    *out = (int)st64;
    return NF_OK;
}

int nf_note_semitones_from_a4(const NfNote *n) {
    int result = 0;
    if (nf_note_checked_semitones_from_a4(n, &result) != NF_OK) {
        return 0;
    }
    return result;
}

NfStatus nf_validate_frequency(double freq_hz) {
    if (!is_finite_dbl(freq_hz) || freq_hz <= 0.0 ||
        freq_hz > NF_MAX_FREQUENCY_HZ) {
        return NF_ERR_INVALID_FREQUENCY;
    }
    return NF_OK;
}

NfStatus nf_note_frequency_with_reference(const NfNote *n, double a4_hz,
                                          double *out) {
    int64_t st64;
    NfStatus status;
    double exponent;
    double freq;
    if (n == NULL || out == NULL) {
        return NF_ERR_NULL_POINTER;
    }
    if (!is_finite_dbl(a4_hz) || a4_hz <= 0.0 ||
        a4_hz > NF_MAX_REFERENCE_FREQUENCY_HZ) {
        return NF_ERR_INVALID_FREQUENCY;
    }
    status = checked_semitones_i64(n, &st64);
    if (status != NF_OK) {
        return status;
    }
    exponent = (double)st64 / (double)NF_SEMITONES_PER_OCTAVE;
    freq = a4_hz * exp2_d(exponent);
    if (nf_validate_frequency(freq) != NF_OK) {
        return NF_ERR_INVALID_FREQUENCY;
    }
    *out = freq;
    return NF_OK;
}

NfStatus nf_note_checked_frequency(const NfNote *n, double *out) {
    return nf_note_frequency_with_reference(n, NF_REFERENCE_FREQUENCY_HZ, out);
}

double nf_note_frequency(const NfNote *n) {
    double freq = 0.0;
    if (nf_note_checked_frequency(n, &freq) != NF_OK) {
        return 0.0;
    }
    return freq;
}

void nf_note_to_string(const NfNote *n, char *buf, size_t bufsz) {
    if (buf == NULL || bufsz == 0) {
        return;
    }
    if (nf_note_chromatic_index(n) < 0 || n->octave < NF_MIN_OCTAVE ||
        n->octave > NF_MAX_OCTAVE) {
        buf[0] = '\0';
        return;
    }
    snprintf(buf, bufsz, "%c%s%d", n->letter, n->accidental, n->octave);
}

/* ---------------------------------------------------------------------------
 *  Parsing
 * ------------------------------------------------------------------------- */

/* Parse an optional '-' followed by one or more ASCII digits into *out,
 * bounded to [NF_MIN_OCTAVE, NF_MAX_OCTAVE]. Returns 0 on success, -1 on error. */
static int parse_octave(const char *text, int *out) {
    int neg = 0;
    int val = 0;
    const char *p = text;
    if (text == NULL || out == NULL || *p == '\0') {
        return -1;
    }
    if (*p == '-') {
        neg = 1;
        p++;
    }
    if (*p == '\0') {
        return -1; /* bare "-" */
    }
    for (; *p != '\0'; p++) {
        if (*p < '0' || *p > '9') {
            return -1;
        }
        val = val * 10 + (*p - '0');
        /* Since |NF_MIN_OCTAVE| == NF_MAX_OCTAVE == 100, exceeding 100 guarantees
         * the parsed octave is out of bounds and prevents any int overflow. */
        if (val > NF_MAX_OCTAVE) {
            return -1;
        }
    }
    if (neg) {
        val = -val;
    }
    if (val < NF_MIN_OCTAVE || val > NF_MAX_OCTAVE) {
        return -1;
    }
    *out = val;
    return 0;
}

NfStatus nf_parse_note(const char *text, NfNote *out) {
    size_t len;
    char letter;
    char up;
    const char *rest;
    const char *accidental;
    const char *octave_text;
    int octave;
    char letter_str[2];

    if (text == NULL || out == NULL) {
        return NF_ERR_NULL_POINTER;
    }

    /* Enforce MAX_NOTE_STRING_LENGTH (32) before parsing (CWE-400). */
    len = bounded_strlen(text, NF_MAX_NOTE_STRING_LENGTH);
    if (len == 0 || len > NF_MAX_NOTE_STRING_LENGTH) {
        return NF_ERR_INVALID_NOTE;
    }

    letter = text[0];
    up = upper_ascii(letter);
    if (up < 'A' || up > 'G') {
        return NF_ERR_INVALID_NOTE;
    }

    rest = text + 1;
    accidental = "";
    if (rest[0] == '#') {
        accidental = "#";
        octave_text = rest + 1;
    } else if (rest[0] == 'b') {
        accidental = "b";
        octave_text = rest + 1;
    } else {
        octave_text = rest;
    }

    if (parse_octave(octave_text, &octave) != 0) {
        return NF_ERR_INVALID_NOTE;
    }

    letter_str[0] = letter;
    letter_str[1] = '\0';
    return nf_note_new(letter_str, accidental, octave, out);
}

NfStatus nf_note_to_frequency_with_reference(const char *text, double a4_hz,
                                             double *out) {
    NfNote n;
    NfStatus st;
    if (text == NULL || out == NULL) {
        return NF_ERR_NULL_POINTER;
    }
    st = nf_parse_note(text, &n);
    if (st != NF_OK) {
        return st;
    }
    return nf_note_frequency_with_reference(&n, a4_hz, out);
}

NfStatus nf_note_to_frequency(const char *text, double *out) {
    return nf_note_to_frequency_with_reference(text, NF_REFERENCE_FREQUENCY_HZ,
                                               out);
}
