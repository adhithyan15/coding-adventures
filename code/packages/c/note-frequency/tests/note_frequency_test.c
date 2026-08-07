/*
 * Tests for the C note-frequency library, using the header-only iso_test.h
 * harness (pure ISO). Reference frequencies are the standard 12-TET pitches.
 */
#include "iso_test.h"

#include <string.h>

#include "note_frequency.h"

/* Parse `text` and assert its frequency is `hz` within `eps`. */
static void expect_freq(const char *text, double hz, double eps) {
    double f;
    ISO_CHECK(nf_note_to_frequency(text, &f) == NF_OK);
    ISO_CHECK_EQ_DBL(f, hz, eps);
}

int main(void) {
    const double eps = 1e-6;

    /* ── reference pitches ───────────────────────────────────────────────── */
    expect_freq("A4", 440.0, eps);   /* concert A, exact */
    expect_freq("A3", 220.0, eps);   /* one octave down */
    expect_freq("A5", 880.0, eps);   /* one octave up */
    expect_freq("C4", 261.6255653005986, eps);  /* middle C */
    expect_freq("E4", 329.6275569128699, eps);
    expect_freq("C#5", 554.3652619537442, eps);
    expect_freq("Db3", 138.59131548843604, eps);
    expect_freq("A-1", 13.75, eps);  /* negative octave */

    /* ── enharmonic spellings share a pitch ──────────────────────────────── */
    {
        double a, b;
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

    /* ── nf_note_new direct, including an invalid spelling ───────────────── */
    {
        NfNote n;
        ISO_CHECK(nf_note_new("c", "#", 4, &n) == NF_OK);
        ISO_CHECK(n.letter == 'C');
        ISO_CHECK_STR_EQ(n.accidental, "#");
        /* "Cb", "E#", "B#", "Fb" are not real chromatic spellings. */
        ISO_CHECK(nf_note_new("C", "b", 3, &n) == NF_ERR_INVALID_SPELLING);
        ISO_CHECK(nf_note_new("E", "#", 3, &n) == NF_ERR_INVALID_SPELLING);
        ISO_CHECK(nf_note_new("B", "#", 3, &n) == NF_ERR_INVALID_SPELLING);
    }

    /* ── parse errors ────────────────────────────────────────────────────── */
    {
        NfNote n;
        ISO_CHECK(nf_parse_note("", &n) == NF_ERR_INVALID_NOTE);
        ISO_CHECK(nf_parse_note("H4", &n) == NF_ERR_INVALID_NOTE); /* no letter H */
        ISO_CHECK(nf_parse_note("A", &n) == NF_ERR_INVALID_NOTE);  /* no octave */
        ISO_CHECK(nf_parse_note("A#", &n) == NF_ERR_INVALID_NOTE); /* no octave */
        ISO_CHECK(nf_parse_note("Ax4", &n) == NF_ERR_INVALID_NOTE); /* bad accidental */
        ISO_CHECK(nf_parse_note("A#b3", &n) == NF_ERR_INVALID_NOTE);/* octave not digits */
        /* "Cb3" parses structurally but "Cb" is an invalid spelling. */
        ISO_CHECK(nf_parse_note("Cb3", &n) == NF_ERR_INVALID_SPELLING);
    }

    /* ── extreme octaves stay defined (semitone math done in 64-bit) ─────── */
    {
        /* octave 2e8 would overflow int32 in (octave-4)*12; the wide math keeps
         * it UB-free. The frequency saturates to +inf, negative to ~0. */
        double f;
        ISO_CHECK(nf_note_to_frequency("A200000000", &f) == NF_OK);
        ISO_CHECK(f > 1e300);
        ISO_CHECK(nf_note_to_frequency("A-200000000", &f) == NF_OK);
        ISO_CHECK(f >= 0.0 && f < 1e-30);
    }

    return ISO_TEST_RESULT();
}
