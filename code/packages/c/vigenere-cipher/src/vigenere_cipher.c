/*
 * vigenere_cipher.c — implementation of the Vigenere cipher and its
 * cryptanalysis (see vigenere_cipher.h). A faithful port of the Rust
 * `vigenere-cipher` crate (cipher.rs + analysis.rs).
 */
#include "vigenere_cipher.h"

#include <float.h>  /* DBL_MAX */
#include <stdint.h> /* SIZE_MAX */
#include <stdlib.h> /* malloc, calloc, free */
#include <string.h> /* strlen */

/* ---- ASCII helpers (deterministic, locale-independent) ---------------- */

static int is_upper(int c) { return c >= 'A' && c <= 'Z'; }
static int is_lower(int c) { return c >= 'a' && c <= 'z'; }
static int is_alpha(int c) { return is_upper(c) || is_lower(c); }
static unsigned char to_upper(unsigned char c) {
    return is_lower(c) ? (unsigned char)(c - 'a' + 'A') : c;
}

/* English letter frequencies (A-Z) for chi-squared analysis. */
static const double ENGLISH_FREQUENCIES[26] = {
    0.08167, 0.01492, 0.02782, 0.04253, 0.12702, 0.02228, 0.02015,
    0.06094, 0.06966, 0.00153, 0.00772, 0.04025, 0.02406, 0.06749,
    0.07507, 0.01929, 0.00095, 0.05987, 0.06327, 0.09056, 0.02758,
    0.00978, 0.02360, 0.00150, 0.01974, 0.00074};

/* ---- core cipher ------------------------------------------------------ */

int vigenere_key_valid(const char *key) {
    size_t i;
    if (key[0] == '\0') {
        return 0;
    }
    for (i = 0; key[i]; i++) {
        if (!is_alpha((unsigned char)key[i])) {
            return 0;
        }
    }
    return 1;
}

/* Shared engine for encrypt (forward = 1) and decrypt (forward = 0). */
static char *transform(const char *text, const char *key, int forward) {
    size_t klen, tlen, i, ki = 0;
    unsigned char *kup;
    char *result;

    if (!vigenere_key_valid(key)) {
        return NULL;
    }
    klen = strlen(key);
    tlen = strlen(text);

    kup = malloc(klen);
    if (!kup) {
        return NULL;
    }
    for (i = 0; i < klen; i++) {
        kup[i] = to_upper((unsigned char)key[i]);
    }

    result = malloc(tlen + 1); /* tlen < SIZE_MAX since it is a strlen */
    if (!result) {
        free(kup);
        return NULL;
    }
    for (i = 0; i < tlen; i++) {
        unsigned char ch = (unsigned char)text[i];
        if (is_upper(ch) || is_lower(ch)) {
            int base = is_upper(ch) ? 'A' : 'a';
            int shift = kup[ki % klen] - 'A';
            int off = forward ? (ch - base + shift)
                              : (ch - base + 26 - shift);
            result[i] = (char)((off % 26) + base);
            ki++;
        } else {
            result[i] = (char)ch; /* non-alpha passes through; key stays put */
        }
    }
    result[tlen] = '\0';
    free(kup);
    return result;
}

char *vigenere_encrypt(const char *plaintext, const char *key) {
    return transform(plaintext, key, 1);
}

char *vigenere_decrypt(const char *ciphertext, const char *key) {
    return transform(ciphertext, key, 0);
}

/* ---- cryptanalysis ---------------------------------------------------- */

/* Extract the ASCII letters of `text`, upper-cased, into a fresh buffer; sets
 * *n_out to the count. Returns NULL only on allocation failure. */
static unsigned char *extract_alpha_upper(const char *text, size_t *n_out) {
    size_t len = strlen(text), count = 0, i, k = 0;
    unsigned char *out;
    for (i = 0; i < len; i++) {
        if (is_alpha((unsigned char)text[i])) {
            count++;
        }
    }
    *n_out = count;
    out = malloc(count ? count : 1); /* never malloc(0) */
    if (!out) {
        return NULL;
    }
    for (i = 0; i < len; i++) {
        unsigned char c = (unsigned char)text[i];
        if (is_alpha(c)) {
            out[k++] = to_upper(c);
        }
    }
    return out;
}

/* Chi-squared of `counts` (26 bins, `total` letters) against English. The
 * caller guarantees total > 0, so every expected value is > 0. */
static double chi_squared(const size_t counts[26], size_t total) {
    double chi2 = 0.0;
    int i;
    for (i = 0; i < 26; i++) {
        double expected = ENGLISH_FREQUENCIES[i] * (double)total;
        double diff = (double)counts[i] - expected;
        chi2 += (diff * diff) / expected;
    }
    return chi2;
}

size_t vigenere_find_key_length(const char *ciphertext, size_t max_length) {
    size_t n, limit, k, i, result = 1;
    unsigned char *letters;
    double *avg_ics;
    double best_ic, threshold;

    letters = extract_alpha_upper(ciphertext, &n);
    if (!letters) {
        return 1; /* the crate cannot fail here; degrade gracefully */
    }
    if (n < 2) {
        free(letters);
        return 1;
    }

    limit = (max_length < n / 2) ? max_length : n / 2;
    avg_ics = calloc(limit + 1, sizeof *avg_ics); /* checked multiply */
    if (!avg_ics) {
        free(letters);
        return 1;
    }

    /* Average Index of Coincidence of the k position-groups, for each k. */
    for (k = 2; k <= limit; k++) {
        double total_ic = 0.0;
        size_t group_count = 0;
        for (i = 0; i < k; i++) {
            size_t counts[26] = {0}, gn = 0, j, num = 0, t;
            for (j = i; j < n; j += k) {
                counts[letters[j] - 'A']++;
                gn++;
            }
            if (gn > 1) {
                for (t = 0; t < 26; t++) {
                    num += counts[t] * (counts[t] > 0 ? counts[t] - 1 : 0);
                }
                total_ic += (double)num / (double)(gn * (gn - 1));
                group_count++;
            }
        }
        if (group_count > 0) {
            avg_ics[k] = total_ic / (double)group_count;
        }
    }

    best_ic = 0.0;
    for (k = 0; k <= limit; k++) {
        if (avg_ics[k] > best_ic) {
            best_ic = avg_ics[k];
        }
    }
    if (best_ic > 0.0) {
        threshold = best_ic * 0.9;
        for (k = 2; k <= limit; k++) {
            if (avg_ics[k] >= threshold) {
                result = k;
                break;
            }
        }
    }

    free(letters);
    free(avg_ics);
    return result;
}

char *vigenere_find_key(const char *ciphertext, size_t key_length) {
    size_t n, pos;
    unsigned char *letters;
    char *key;

    if (key_length == SIZE_MAX) {
        return NULL; /* key_length + 1 would overflow */
    }
    letters = extract_alpha_upper(ciphertext, &n);
    if (!letters) {
        return NULL;
    }
    key = malloc(key_length + 1);
    if (!key) {
        free(letters);
        return NULL;
    }

    for (pos = 0; pos < key_length; pos++) {
        size_t gn = 0, j;
        unsigned int shift, best_shift = 0;
        double best_chi2 = DBL_MAX;
        for (j = pos; j < n; j += key_length) { /* key_length >= 1 here */
            gn++;
        }
        if (gn == 0) {
            key[pos] = 'A';
            continue;
        }
        for (shift = 0; shift < 26; shift++) {
            size_t counts[26] = {0};
            double chi2;
            for (j = pos; j < n; j += key_length) {
                unsigned int dec = (letters[j] - 'A' + 26 - shift) % 26;
                counts[dec]++;
            }
            chi2 = chi_squared(counts, gn);
            if (chi2 < best_chi2) {
                best_chi2 = chi2;
                best_shift = shift;
            }
        }
        key[pos] = (char)('A' + best_shift);
    }
    key[key_length] = '\0';

    free(letters);
    return key;
}

int vigenere_break(const char *ciphertext, VigenereBreak *out) {
    size_t klen = vigenere_find_key_length(ciphertext, 20);
    char *key = vigenere_find_key(ciphertext, klen);
    char *plaintext;
    if (!key) {
        return 0;
    }
    plaintext = vigenere_decrypt(ciphertext, key);
    if (!plaintext) {
        /* The recovered key is all upper-case letters, so decrypt only fails on
         * allocation failure; mirror Rust's unwrap_or_default with "". */
        plaintext = malloc(1);
        if (!plaintext) {
            free(key);
            return 0;
        }
        plaintext[0] = '\0';
    }
    out->key = key;
    out->plaintext = plaintext;
    return 1;
}

void vigenere_break_free(VigenereBreak *r) {
    if (!r) {
        return;
    }
    free(r->key);
    free(r->plaintext);
    r->key = NULL;
    r->plaintext = NULL;
}
