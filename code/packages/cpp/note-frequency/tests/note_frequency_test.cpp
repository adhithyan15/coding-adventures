// Tests for the C++ note-frequency library, using the header-only iso_test.h
// harness (pure ISO). Reference frequencies are the standard 12-TET pitches.
#include "iso_test.h"

#include <stdexcept>
#include <string>

#include "note_frequency.hpp"

namespace nf = ca::note_frequency;

// True if calling `fn` throws std::invalid_argument.
template <typename F>
static bool throws_invalid(F fn) {
    try {
        fn();
    } catch (const std::invalid_argument&) {
        return true;
    }
    return false;
}

int main() {
    const double eps = 1e-6;

    // ── reference pitches ─────────────────────────────────────────────────
    ISO_CHECK_EQ_DBL(nf::note_to_frequency("A4"), 440.0, eps);
    ISO_CHECK_EQ_DBL(nf::note_to_frequency("A3"), 220.0, eps);
    ISO_CHECK_EQ_DBL(nf::note_to_frequency("A5"), 880.0, eps);
    ISO_CHECK_EQ_DBL(nf::note_to_frequency("C4"), 261.6255653005986, eps);
    ISO_CHECK_EQ_DBL(nf::note_to_frequency("E4"), 329.6275569128699, eps);
    ISO_CHECK_EQ_DBL(nf::note_to_frequency("C#5"), 554.3652619537442, eps);
    ISO_CHECK_EQ_DBL(nf::note_to_frequency("Db3"), 138.59131548843604, eps);
    ISO_CHECK_EQ_DBL(nf::note_to_frequency("A-1"), 13.75, eps);

    // ── enharmonic spellings share a pitch; lower-case letter accepted ─────
    ISO_CHECK_EQ_DBL(nf::note_to_frequency("C#4"),
                     nf::note_to_frequency("Db4"), eps);
    ISO_CHECK_EQ_DBL(nf::note_to_frequency("a4"), 440.0, eps);

    // ── semitone distance from A4 ─────────────────────────────────────────
    ISO_CHECK_EQ_INT(nf::parse_note("A4").semitones_from_a4(), 0);
    ISO_CHECK_EQ_INT(nf::parse_note("C4").semitones_from_a4(), -9);
    ISO_CHECK_EQ_INT(nf::parse_note("C5").semitones_from_a4(), 3);
    ISO_CHECK_EQ_INT(nf::parse_note("A3").semitones_from_a4(), -12);

    // ── chromatic index, spelling, Display ────────────────────────────────
    {
        nf::Note n = nf::parse_note("F#3");
        ISO_CHECK_EQ_INT(n.chromatic_index(), 6);
        ISO_CHECK_STR_EQ(n.spelling().c_str(), "F#");
        ISO_CHECK_STR_EQ(n.to_string().c_str(), "F#3");
        ISO_CHECK_STR_EQ(n.letter().c_str(), "F");
        ISO_CHECK_EQ_INT(n.octave(), 3);
    }

    // ── Note constructor, including invalid spellings ─────────────────────
    {
        nf::Note c(std::string("c"), "#", 4);  // canonicalized to C#
        ISO_CHECK_STR_EQ(c.spelling().c_str(), "C#");
        ISO_CHECK(throws_invalid([] { nf::Note(std::string("C"), "b", 3); }));
        ISO_CHECK(throws_invalid([] { nf::Note(std::string("E"), "#", 3); }));
        ISO_CHECK(throws_invalid([] { nf::Note(std::string("B"), "#", 3); }));
    }

    // ── parse errors throw std::invalid_argument ──────────────────────────
    ISO_CHECK(throws_invalid([] { nf::parse_note(""); }));
    ISO_CHECK(throws_invalid([] { nf::parse_note("H4"); }));
    ISO_CHECK(throws_invalid([] { nf::parse_note("A"); }));
    ISO_CHECK(throws_invalid([] { nf::parse_note("A#"); }));
    ISO_CHECK(throws_invalid([] { nf::parse_note("Ax4"); }));
    ISO_CHECK(throws_invalid([] { nf::parse_note("A#b3"); }));
    ISO_CHECK(throws_invalid([] { nf::parse_note("Cb3"); }));  // invalid spelling

    // ── equality ──────────────────────────────────────────────────────────
    ISO_CHECK(nf::parse_note("A4") == nf::parse_note("A4"));
    ISO_CHECK(nf::parse_note("A4") != nf::parse_note("A5"));

    // ── extreme octaves stay defined (semitone math done in 64-bit) ───────
    ISO_CHECK(nf::note_to_frequency("A200000000") > 1e300);   // saturates +inf
    {
        double f = nf::note_to_frequency("A-200000000");
        ISO_CHECK(f >= 0.0 && f < 1e-30);                     // saturates 0
    }

    return ISO_TEST_RESULT();
}
