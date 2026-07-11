/*
 * caesar_cipher.c — implementation of the Caesar cipher and its
 * frequency-analysis attack. Ported from the Rust `caesar-cipher` crate; the
 * algorithms (shift normalisation, chi-squared scoring, English frequency
 * table) match it exactly.
 */
#include "caesar_cipher.h"

#include <float.h>  /* DBL_MAX — the "no letters" sentinel */
#include <string.h> /* strlen */

/* Standard English letter frequencies (A..Z), summing to ~1.0. Identical to the
 * table in the Rust crate. Used to score how "English-like" a decryption is. */
static const double ENGLISH_FREQUENCIES[26] = {
    0.08167, /* A */ 0.01492, /* B */ 0.02782, /* C */ 0.04253, /* D */
    0.12702, /* E */ 0.02228, /* F */ 0.02015, /* G */ 0.06094, /* H */
    0.06966, /* I */ 0.00153, /* J */ 0.00772, /* K */ 0.04025, /* L */
    0.02406, /* M */ 0.06749, /* N */ 0.07507, /* O */ 0.01929, /* P */
    0.00095, /* Q */ 0.05987, /* R */ 0.06327, /* S */ 0.09056, /* T */
    0.02758, /* U */ 0.00978, /* V */ 0.02360, /* W */ 0.00150, /* X */
    0.01974, /* Y */ 0.00074  /* Z */
};

/* We deliberately avoid <ctype.h> is_* functions: they take an int whose value
 * must be representable as unsigned char, and their behavior depends on the
 * locale. Restricting ourselves to the ASCII A–Z / a–z ranges keeps the cipher
 * deterministic and locale-independent, exactly like the Rust version. */

static int is_upper_ascii(char ch) {
    return ch >= 'A' && ch <= 'Z';
}

static int is_lower_ascii(char ch) {
    return ch >= 'a' && ch <= 'z';
}

/* shift_char — map one character by an already-normalised shift (0..25). */
static char shift_char(char ch, int normalised_shift) {
    if (is_upper_ascii(ch)) {
        int position = ch - 'A';
        int new_position = (position + normalised_shift) % 26;
        return (char)('A' + new_position);
    }
    if (is_lower_ascii(ch)) {
        int position = ch - 'a';
        int new_position = (position + normalised_shift) % 26;
        return (char)('a' + new_position);
    }
    return ch; /* non-letters pass through unchanged */
}

/* Normalise any (possibly negative or large) shift into 0..25.
 * ((shift % 26) + 26) % 26 handles negatives portably. */
static int normalise_shift(int shift) {
    return ((shift % 26) + 26) % 26;
}

long caesar_encrypt(const char *text, int shift, char *out, size_t out_size) {
    size_t len = strlen(text);
    int normalised;
    size_t i;

    if (out_size < len + 1) {
        return -1; /* not enough room for the text plus its NUL terminator */
    }
    normalised = normalise_shift(shift);
    for (i = 0; i < len; i++) {
        out[i] = shift_char(text[i], normalised);
    }
    out[len] = '\0';
    return (long)len;
}

long caesar_decrypt(const char *text, int shift, char *out, size_t out_size) {
    /* Decrypting by `shift` is encrypting by `-shift`. */
    return caesar_encrypt(text, -shift, out, out_size);
}

long caesar_rot13(const char *text, char *out, size_t out_size) {
    return caesar_encrypt(text, 13, out, out_size);
}

void caesar_letter_counts(const char *text, size_t counts[26]) {
    size_t i;
    for (i = 0; i < 26; i++) {
        counts[i] = 0;
    }
    for (i = 0; text[i] != '\0'; i++) {
        char ch = text[i];
        if (is_upper_ascii(ch)) {
            counts[ch - 'A']++;
        } else if (is_lower_ascii(ch)) {
            counts[ch - 'a']++;
        }
    }
}

double caesar_chi_squared(const char *text) {
    size_t counts[26];
    size_t total = 0;
    double total_f;
    double sum = 0.0;
    size_t i;

    caesar_letter_counts(text, counts);
    for (i = 0; i < 26; i++) {
        total += counts[i];
    }
    if (total == 0) {
        return DBL_MAX; /* no letters — no frequency signal */
    }
    total_f = (double)total;
    for (i = 0; i < 26; i++) {
        double expected = total_f * ENGLISH_FREQUENCIES[i];
        if (expected < 1e-10) {
            continue; /* guard against division by zero (never happens here) */
        }
        {
            double diff = (double)counts[i] - expected;
            sum += diff * diff / expected;
        }
    }
    return sum;
}

int caesar_frequency_analysis(const char *ciphertext, char *out,
                              size_t out_size) {
    /* Seed with shift 1 so that even when every candidate ties (e.g. no letters
     * at all), we still return a valid shift and its decryption. */
    int best_shift = 1;
    double best_score;
    int shift;

    /* First pass: reuse the caller's buffer as scratch to score every shift.
     * (No fixed-size local buffer, so arbitrarily long ciphertext works as long
     * as `out` fits it — the same requirement as any single decrypt.) */
    if (caesar_decrypt(ciphertext, 1, out, out_size) < 0) {
        return -1;
    }
    best_score = caesar_chi_squared(out);

    for (shift = 2; shift <= 25; shift++) {
        double score;
        if (caesar_decrypt(ciphertext, shift, out, out_size) < 0) {
            return -1;
        }
        score = caesar_chi_squared(out);
        if (score < best_score) {
            best_score = score;
            best_shift = shift;
        }
    }

    /* `out` currently holds shift 25's decryption; re-decrypt the winner. */
    if (caesar_decrypt(ciphertext, best_shift, out, out_size) < 0) {
        return -1;
    }
    return best_shift;
}
