/*
 * Tests for the C note-frequency library, using the header-only iso_test.h
 * harness (pure ISO). Includes reference 12-TET tests, VER15 hostile/adversarial
 * tests (CWE-20, CWE-190, CWE-400, CWE-476), and SplitMix64 PBT tests.
 */
#include "iso_test.h"

#include <limits.h>
#include <stdint.h>
#include <string.h>

#include "note_frequency.h"

static double dbl_from_bits(uint64_t bits) {
    double d = 0.0;
    memcpy(&d, &bits, sizeof d);
    return d;
}

static double make_nan(void) { return dbl_from_bits(UINT64_C(0x7FF8000000000000)); }
static double make_inf(void) { return dbl_from_bits(UINT64_C(0x7FF0000000000000)); }
static double make_neg_inf(void) { return dbl_from_bits(UINT64_C(0xFFF0000000000000)); }

static uint64_t splitmix64(uint64_t *state) {
    uint64_t z = (*state += UINT64_C(0x9e3779b97f4a7c15));
    z = (z ^ (z >> 30)) * UINT64_C(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)) * UINT64_C(0x94d049bb133111eb);
    return z ^ (z >> 31);
}

static int approx_rel(double a, double b, double rel_tol) {
    double diff = a - b;
    double scale = a < 0.0 ? -a : a;
    double b_abs = b < 0.0 ? -b : b;
    if (diff < 0.0) {
        diff = -diff;
    }
    if (b_abs > scale) {
        scale = b_abs;
    }
    if (scale < 1.0) {
        scale = 1.0;
    }
    return (a == b) || (diff <= scale * rel_tol);
}

/* Parse `text` and assert its frequency is `hz` within `eps`. */
static void expect_freq(const char *text, double hz, double eps) {
    double f = 0.0;
    ISO_CHECK(nf_note_to_frequency(text, &f) == NF_OK);
    ISO_CHECK_EQ_DBL(f, hz, eps);
}

static void test_reference_and_core(void) {
    const double eps = 1e-6;

    /* ── reference pitches ───────────────────────────────────────────────── */
    expect_freq("A4", 440.0, eps);
    expect_freq("A3", 220.0, eps);
    expect_freq("A5", 880.0, eps);
    expect_freq("C4", 261.6255653005986, eps);
    expect_freq("E4", 329.6275569128699, eps);
    expect_freq("C#5", 554.3652619537442, eps);
    expect_freq("Db3", 138.59131548843604, eps);
    expect_freq("A-1", 13.75, eps);

    /* ── enharmonic spellings share a pitch ──────────────────────────────── */
    {
        double a = 0.0, b = 0.0;
        ISO_CHECK(nf_note_to_frequency("C#4", &a) == NF_OK);
        ISO_CHECK(nf_note_to_frequency("Db4", &b) == NF_OK);
        ISO_CHECK_EQ_DBL(a, b, eps);
    }

    /* ── lower-case letter is accepted (canonicalized) ───────────────────── */
    expect_freq("a4", 440.0, eps);

    /* ── semitone distance from A4 ───────────────────────────────────────── */
    {
        NfNote n;
        ISO_CHECK(nf_parse_note("A4", &n) == NF_OK);
        ISO_CHECK_EQ_INT(nf_note_semitones_from_a4(&n), 0);
        ISO_CHECK(nf_parse_note("C4", &n) == NF_OK);
        ISO_CHECK_EQ_INT(nf_note_semitones_from_a4(&n), -9);
        ISO_CHECK(nf_parse_note("C5", &n) == NF_OK);
        ISO_CHECK_EQ_INT(nf_note_semitones_from_a4(&n), 3);
        ISO_CHECK(nf_parse_note("A3", &n) == NF_OK);
        ISO_CHECK_EQ_INT(nf_note_semitones_from_a4(&n), -12);
    }

    /* ── chromatic index, spelling, and Display ──────────────────────────── */
    {
        NfNote n;
        char buf[16];
        ISO_CHECK(nf_parse_note("F#3", &n) == NF_OK);
        ISO_CHECK_EQ_INT(nf_note_chromatic_index(&n), 6);
        nf_note_spelling(&n, buf, sizeof buf);
        ISO_CHECK_STR_EQ(buf, "F#");
        nf_note_to_string(&n, buf, sizeof buf);
        ISO_CHECK_STR_EQ(buf, "F#3");
        ISO_CHECK(n.letter == 'F');
        ISO_CHECK_EQ_INT(n.octave, 3);
    }

    /* ── nf_note_new direct, including invalid spellings ─────────────────── */
    {
        NfNote n;
        ISO_CHECK(nf_note_new("c", "#", 4, &n) == NF_OK);
        ISO_CHECK(n.letter == 'C');
        ISO_CHECK_STR_EQ(n.accidental, "#");
        ISO_CHECK(nf_note_new("C", "b", 3, &n) == NF_ERR_INVALID_SPELLING);
        ISO_CHECK(nf_note_new("E", "#", 3, &n) == NF_ERR_INVALID_SPELLING);
        ISO_CHECK(nf_note_new("B", "#", 3, &n) == NF_ERR_INVALID_SPELLING);
        ISO_CHECK(nf_note_new("F", "b", 3, &n) == NF_ERR_INVALID_SPELLING);
    }

    /* ── parse errors ────────────────────────────────────────────────────── */
    {
        NfNote n;
        ISO_CHECK(nf_parse_note("", &n) == NF_ERR_INVALID_NOTE);
        ISO_CHECK(nf_parse_note("H4", &n) == NF_ERR_INVALID_NOTE);
        ISO_CHECK(nf_parse_note("A", &n) == NF_ERR_INVALID_NOTE);
        ISO_CHECK(nf_parse_note("A#", &n) == NF_ERR_INVALID_NOTE);
        ISO_CHECK(nf_parse_note("Ax4", &n) == NF_ERR_INVALID_NOTE);
        ISO_CHECK(nf_parse_note("A#b3", &n) == NF_ERR_INVALID_NOTE);
        ISO_CHECK(nf_parse_note("Cb3", &n) == NF_ERR_INVALID_SPELLING);
    }

    /* ── octave bounds [-100, 100] ───────────────────────────────────────── */
    {
        double f = 0.0;
        ISO_CHECK(nf_note_to_frequency("A100", &f) == NF_OK);
        ISO_CHECK(nf_validate_frequency(f) == NF_OK);
        ISO_CHECK(nf_note_to_frequency("A-100", &f) == NF_OK);
        ISO_CHECK(nf_validate_frequency(f) == NF_OK);
        ISO_CHECK(nf_note_to_frequency("B100", &f) == NF_OK);
        ISO_CHECK(nf_validate_frequency(f) == NF_OK);
        ISO_CHECK(nf_note_to_frequency("C-100", &f) == NF_OK);
        ISO_CHECK(nf_validate_frequency(f) == NF_OK);

        ISO_CHECK(nf_note_to_frequency("A101", &f) == NF_ERR_INVALID_NOTE);
        ISO_CHECK(nf_note_to_frequency("A-101", &f) == NF_ERR_INVALID_NOTE);
        ISO_CHECK(nf_note_to_frequency("A200000000", &f) == NF_ERR_INVALID_NOTE);
        ISO_CHECK(nf_note_to_frequency("A-200000000", &f) == NF_ERR_INVALID_NOTE);
    }
}

static void test_null_pointers(void) {
    NfNote n;
    char buf[16];
    int st = 0;
    double f = 0.0;

    ISO_CHECK(nf_parse_note("A4", &n) == NF_OK);

    /* CWE-476: nf_note_new */
    ISO_CHECK(nf_note_new(NULL, "", 4, &n) == NF_ERR_NULL_POINTER);
    ISO_CHECK(nf_note_new("A", NULL, 4, &n) == NF_ERR_NULL_POINTER);
    ISO_CHECK(nf_note_new("A", "", 4, NULL) == NF_ERR_NULL_POINTER);

    /* CWE-476: nf_note_spelling & nf_note_to_string */
    buf[0] = 'X';
    nf_note_spelling(NULL, buf, sizeof buf);
    ISO_CHECK_STR_EQ(buf, "");
    nf_note_spelling(&n, NULL, 16);
    nf_note_spelling(&n, buf, 0);
    {
        NfNote sharp;
        char tiny[2];
        ISO_CHECK(nf_parse_note("C#4", &sharp) == NF_OK);
        nf_note_spelling(&sharp, tiny, 1);
        ISO_CHECK_STR_EQ(tiny, "");
        nf_note_spelling(&sharp, tiny, 2);
        ISO_CHECK_STR_EQ(tiny, "C");
    }

    buf[0] = 'X';
    nf_note_to_string(NULL, buf, sizeof buf);
    ISO_CHECK_STR_EQ(buf, "");
    nf_note_to_string(&n, NULL, 16);
    nf_note_to_string(&n, buf, 0);

    /* CWE-476: query functions */
    ISO_CHECK_EQ_INT(nf_note_chromatic_index(NULL), -1);
    ISO_CHECK_EQ_INT(nf_note_semitones_from_a4(NULL), 0);
    ISO_CHECK(nf_note_checked_semitones_from_a4(NULL, &st) == NF_ERR_NULL_POINTER);
    ISO_CHECK(nf_note_checked_semitones_from_a4(&n, NULL) == NF_ERR_NULL_POINTER);
    ISO_CHECK_EQ_DBL(nf_note_frequency(NULL), 0.0, 0.0);
    ISO_CHECK(nf_note_checked_frequency(NULL, &f) == NF_ERR_NULL_POINTER);
    ISO_CHECK(nf_note_checked_frequency(&n, NULL) == NF_ERR_NULL_POINTER);
    ISO_CHECK(nf_note_frequency_with_reference(NULL, 440.0, &f) == NF_ERR_NULL_POINTER);
    ISO_CHECK(nf_note_frequency_with_reference(&n, 440.0, NULL) == NF_ERR_NULL_POINTER);

    /* CWE-476: parse & top-level helpers */
    ISO_CHECK(nf_parse_note(NULL, &n) == NF_ERR_NULL_POINTER);
    ISO_CHECK(nf_parse_note("A4", NULL) == NF_ERR_NULL_POINTER);
    ISO_CHECK(nf_note_to_frequency(NULL, &f) == NF_ERR_NULL_POINTER);
    ISO_CHECK(nf_note_to_frequency("A4", NULL) == NF_ERR_NULL_POINTER);
    ISO_CHECK(nf_note_to_frequency_with_reference(NULL, 440.0, &f) == NF_ERR_NULL_POINTER);
    ISO_CHECK(nf_note_to_frequency_with_reference("A4", 440.0, NULL) == NF_ERR_NULL_POINTER);
}

static void test_hostile_inputs(void) {
    NfNote n;
    double f = 0.0;
    int st = 0;

    /* CWE-20: reject multi-character letter strings (no silent truncation) */
    ISO_CHECK(nf_note_new("Cab", "", 4, &n) == NF_ERR_INVALID_SPELLING);
    ISO_CHECK(nf_note_new("C#", "", 4, &n) == NF_ERR_INVALID_SPELLING);
    ISO_CHECK(nf_note_new("AA", "", 4, &n) == NF_ERR_INVALID_SPELLING);
    ISO_CHECK(nf_note_new("C ", "", 4, &n) == NF_ERR_INVALID_SPELLING);
    ISO_CHECK(nf_note_new("", "", 4, &n) == NF_ERR_INVALID_SPELLING);

    /* CWE-20: reject multi-character / invalid accidentals */
    ISO_CHECK(nf_note_new("C", "##", 4, &n) == NF_ERR_INVALID_SPELLING);
    ISO_CHECK(nf_note_new("C", "#b", 4, &n) == NF_ERR_INVALID_SPELLING);
    ISO_CHECK(nf_note_new("C", "bb", 4, &n) == NF_ERR_INVALID_SPELLING);
    ISO_CHECK(nf_note_new("C", "# ", 4, &n) == NF_ERR_INVALID_SPELLING);
    ISO_CHECK(nf_note_new("C", "x", 4, &n) == NF_ERR_INVALID_SPELLING);

    /* CWE-20 / CWE-190: octave bounds including INT_MIN / INT_MAX */
    ISO_CHECK(nf_note_new("A", "", INT_MIN, &n) == NF_ERR_INVALID_NOTE);
    ISO_CHECK(nf_note_new("A", "", INT_MAX, &n) == NF_ERR_INVALID_NOTE);
    ISO_CHECK(nf_note_new("A", "", -101, &n) == NF_ERR_INVALID_NOTE);
    ISO_CHECK(nf_note_new("A", "", 101, &n) == NF_ERR_INVALID_NOTE);
    ISO_CHECK(nf_note_new("A", "", -100, &n) == NF_OK);
    ISO_CHECK(nf_note_new("A", "", 100, &n) == NF_OK);

    /* CWE-400: oversized strings > NF_MAX_NOTE_STRING_LENGTH (32) */
    {
        char exact32[33];
        char over33[34];
        char huge[2048];
        size_t i;

        exact32[0] = 'A';
        for (i = 1; i < 31; ++i) exact32[i] = '0';
        exact32[31] = '4';
        exact32[32] = '\0';
        ISO_CHECK(nf_parse_note(exact32, &n) == NF_OK);
        ISO_CHECK_EQ_INT(n.octave, 4);

        over33[0] = 'A';
        for (i = 1; i < 32; ++i) over33[i] = '0';
        over33[32] = '4';
        over33[33] = '\0';
        ISO_CHECK(nf_parse_note(over33, &n) == NF_ERR_INVALID_NOTE);

        huge[0] = 'A';
        for (i = 1; i < sizeof(huge) - 1; ++i) huge[i] = '4';
        huge[sizeof(huge) - 1] = '\0';
        ISO_CHECK(nf_parse_note(huge, &n) == NF_ERR_INVALID_NOTE);
        ISO_CHECK(nf_note_new(huge, "", 4, &n) == NF_ERR_INVALID_SPELLING);
        ISO_CHECK(nf_note_new("A", huge, 4, &n) == NF_ERR_INVALID_SPELLING);
    }

    /* CWE-20: trailing garbage, whitespace, signs, overflow strings */
    {
        static const char *const bad_notes[] = {
            "A4 ", " A4", "A4\n", "A4\t", "A4x", "A4#", "A+4", "A--4",
            "A-", "#4", "4A", "A##4", "A-2147483648", "A2147483647",
            "A99999999999999999999999999999", "Cb-100", "E#100"
        };
        size_t i;
        for (i = 0; i < sizeof bad_notes / sizeof bad_notes[0]; ++i) {
            ISO_CHECK(nf_parse_note(bad_notes[i], &n) != NF_OK);
        }
    }

    /* CWE-20 / CWE-125: manually corrupted NfNote structs fail closed */
    {
        NfNote bad;
        char out_buf[16];

        bad.letter = 'A';
        bad.accidental[0] = '#';
        bad.accidental[1] = '#';
        bad.accidental[2] = '#'; /* non-NUL-terminated */
        bad.octave = 4;
        ISO_CHECK_EQ_INT(nf_note_chromatic_index(&bad), -1);
        ISO_CHECK(nf_note_checked_semitones_from_a4(&bad, &st) != NF_OK);
        ISO_CHECK(nf_note_checked_frequency(&bad, &f) != NF_OK);
        out_buf[0] = 'Z';
        nf_note_spelling(&bad, out_buf, sizeof out_buf);
        ISO_CHECK_STR_EQ(out_buf, "");

        bad.accidental[0] = '\0';
        bad.octave = INT_MIN;
        ISO_CHECK(nf_note_checked_semitones_from_a4(&bad, &st) == NF_ERR_INVALID_NOTE);
        ISO_CHECK(nf_note_checked_frequency(&bad, &f) == NF_ERR_INVALID_NOTE);
        ISO_CHECK_EQ_INT(nf_note_semitones_from_a4(&bad), 0);
        ISO_CHECK_EQ_DBL(nf_note_frequency(&bad), 0.0, 0.0);
        out_buf[0] = 'Z';
        nf_note_to_string(&bad, out_buf, sizeof out_buf);
        ISO_CHECK_STR_EQ(out_buf, "");

        bad.octave = INT_MAX;
        ISO_CHECK(nf_note_checked_semitones_from_a4(&bad, &st) == NF_ERR_INVALID_NOTE);
        ISO_CHECK(nf_note_checked_frequency(&bad, &f) == NF_ERR_INVALID_NOTE);
    }

    /* CWE-20: non-finite & out-of-range frequency validation */
    {
        const double nan_v = make_nan();
        const double inf_v = make_inf();
        const double ninf_v = make_neg_inf();

        ISO_CHECK(nf_validate_frequency(440.0) == NF_OK);
        ISO_CHECK(nf_validate_frequency(nan_v) == NF_ERR_INVALID_FREQUENCY);
        ISO_CHECK(nf_validate_frequency(inf_v) == NF_ERR_INVALID_FREQUENCY);
        ISO_CHECK(nf_validate_frequency(ninf_v) == NF_ERR_INVALID_FREQUENCY);
        ISO_CHECK(nf_validate_frequency(0.0) == NF_ERR_INVALID_FREQUENCY);
        ISO_CHECK(nf_validate_frequency(-0.0) == NF_ERR_INVALID_FREQUENCY);
        ISO_CHECK(nf_validate_frequency(-440.0) == NF_ERR_INVALID_FREQUENCY);
        ISO_CHECK(nf_validate_frequency(1e39) == NF_ERR_INVALID_FREQUENCY);
        ISO_CHECK(nf_validate_frequency(1e308) == NF_ERR_INVALID_FREQUENCY);

        ISO_CHECK(nf_parse_note("A5", &n) == NF_OK);
        ISO_CHECK(nf_note_frequency_with_reference(&n, 432.0, &f) == NF_OK);
        ISO_CHECK_EQ_DBL(f, 864.0, 1e-6);

        ISO_CHECK(nf_note_to_frequency_with_reference("A4", nan_v, &f) == NF_ERR_INVALID_FREQUENCY);
        ISO_CHECK(nf_note_to_frequency_with_reference("A4", inf_v, &f) == NF_ERR_INVALID_FREQUENCY);
        ISO_CHECK(nf_note_to_frequency_with_reference("A4", ninf_v, &f) == NF_ERR_INVALID_FREQUENCY);
        ISO_CHECK(nf_note_to_frequency_with_reference("A4", 0.0, &f) == NF_ERR_INVALID_FREQUENCY);
        ISO_CHECK(nf_note_to_frequency_with_reference("A4", -440.0, &f) == NF_ERR_INVALID_FREQUENCY);
        ISO_CHECK(nf_note_to_frequency_with_reference("A4", 1e7, &f) == NF_ERR_INVALID_FREQUENCY);
    }
}

static void test_pbt_splitmix64(void) {
    static const struct {
        char letter;
        const char *acc;
    } spellings[17] = {
        {'C', ""}, {'C', "#"}, {'D', "b"}, {'D', ""}, {'D', "#"}, {'E', "b"},
        {'E', ""}, {'F', ""},  {'F', "#"}, {'G', "b"}, {'G', ""}, {'G', "#"},
        {'A', "b"}, {'A', ""}, {'A', "#"}, {'B', "b"}, {'B', ""}
    };
    uint64_t rng = UINT64_C(0x123456789abcdef0);
    int iter;

    for (iter = 0; iter < 500; ++iter) {
        uint64_t r1 = splitmix64(&rng);
        uint64_t r2 = splitmix64(&rng);
        uint64_t r3 = splitmix64(&rng);
        size_t sp_idx = (size_t)(r1 % 17u);
        int oct = (int)(r2 % 201u) - 100; /* [-100, 100] */
        int lower = (int)(r3 & 1u);
        char letter_buf[2];
        NfNote n, parsed;
        char text[32];
        double f = 0.0;
        double ref_hz;
        double f_ref = 0.0;
        int st1 = 0, st2 = 0;

        letter_buf[0] = lower ? (char)(spellings[sp_idx].letter - 'A' + 'a')
                              : spellings[sp_idx].letter;
        letter_buf[1] = '\0';

        ISO_CHECK(nf_note_new(letter_buf, spellings[sp_idx].acc, oct, &n) == NF_OK);
        nf_note_to_string(&n, text, sizeof text);
        ISO_CHECK(nf_parse_note(text, &parsed) == NF_OK);
        ISO_CHECK(parsed.letter == n.letter);
        ISO_CHECK_STR_EQ(parsed.accidental, n.accidental);
        ISO_CHECK_EQ_INT(parsed.octave, n.octave);

        ISO_CHECK(nf_note_checked_semitones_from_a4(&n, &st1) == NF_OK);
        ISO_CHECK(nf_note_checked_semitones_from_a4(&parsed, &st2) == NF_OK);
        ISO_CHECK_EQ_INT(st1, st2);

        ISO_CHECK(nf_note_checked_frequency(&n, &f) == NF_OK);
        ISO_CHECK(nf_validate_frequency(f) == NF_OK);

        if (oct < NF_MAX_OCTAVE) {
            NfNote up;
            int st_up = 0;
            double f_up = 0.0;
            ISO_CHECK(nf_note_new(letter_buf, spellings[sp_idx].acc, oct + 1, &up) == NF_OK);
            ISO_CHECK(nf_note_checked_semitones_from_a4(&up, &st_up) == NF_OK);
            ISO_CHECK_EQ_INT(st_up - st1, 12);
            ISO_CHECK(nf_note_checked_frequency(&up, &f_up) == NF_OK);
            ISO_CHECK(approx_rel(f_up / f, 2.0, 1e-6));
        }

        ref_hz = 20.0 + (double)(r3 % 19980u);
        ISO_CHECK(nf_note_frequency_with_reference(&n, ref_hz, &f_ref) == NF_OK);
        ISO_CHECK(approx_rel(f_ref / f, ref_hz / 440.0, 1e-6));
    }

    /* Fuzz random ASCII/control strings via SplitMix64 */
    for (iter = 0; iter < 300; ++iter) {
        char fuzz[65];
        size_t len = (size_t)(splitmix64(&rng) % 64u);
        size_t j;
        NfNote out;
        NfStatus st;
        for (j = 0; j < len; ++j) {
            uint64_t b = splitmix64(&rng);
            char ch = (char)(1u + (b % 127u)); /* non-NUL ASCII */
            fuzz[j] = ch;
        }
        fuzz[len] = '\0';

        st = nf_parse_note(fuzz, &out);
        if (st == NF_OK) {
            double f = 0.0;
            ISO_CHECK(len >= 2 && len <= NF_MAX_NOTE_STRING_LENGTH);
            ISO_CHECK(out.octave >= NF_MIN_OCTAVE && out.octave <= NF_MAX_OCTAVE);
            ISO_CHECK(nf_note_chromatic_index(&out) >= 0);
            ISO_CHECK(nf_note_checked_frequency(&out, &f) == NF_OK);
            ISO_CHECK(nf_validate_frequency(f) == NF_OK);
        }
    }
}

int main(void) {
    test_reference_and_core();
    test_null_pointers();
    test_hostile_inputs();
    test_pbt_splitmix64();
    return ISO_TEST_RESULT();
}
