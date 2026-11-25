// Tests for the C++ note-frequency library, using the header-only iso_test.h
// harness (pure ISO). Includes reference 12-TET tests, VER15 hostile/adversarial
// tests (CWE-20, CWE-190, CWE-400, CWE-476), and SplitMix64 PBT tests.
#include "iso_test.h"

#include <climits>
#include <cstdint>
#include <limits>
#include <stdexcept>
#include <string>

#include "note_frequency.hpp"

namespace nf = ca::note_frequency;

namespace {

template <typename F>
bool throws_invalid(F fn) {
    try {
        fn();
    } catch (const std::invalid_argument&) {
        return true;
    }
    return false;
}

std::uint64_t splitmix64(std::uint64_t& state) {
    state += UINT64_C(0x9e3779b97f4a7c15);
    std::uint64_t z = state;
    z = (z ^ (z >> 30)) * UINT64_C(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)) * UINT64_C(0x94d049bb133111eb);
    return z ^ (z >> 31);
}

bool approx_rel(double a, double b, double rel_tol) {
    double diff = a - b;
    if (diff < 0.0) diff = -diff;
    double scale = a < 0.0 ? -a : a;
    double b_abs = b < 0.0 ? -b : b;
    if (b_abs > scale) scale = b_abs;
    if (scale < 1.0) scale = 1.0;
    return (a == b) || (diff <= scale * rel_tol);
}

void test_reference_and_core() {
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
        nf::Note c(std::string("c"), "#", 4);
        ISO_CHECK_STR_EQ(c.spelling().c_str(), "C#");
        ISO_CHECK(throws_invalid([] { nf::Note(std::string("C"), "b", 3); }));
        ISO_CHECK(throws_invalid([] { nf::Note(std::string("E"), "#", 3); }));
        ISO_CHECK(throws_invalid([] { nf::Note(std::string("B"), "#", 3); }));
        ISO_CHECK(throws_invalid([] { nf::Note(std::string("F"), "b", 3); }));
    }

    // ── parse errors throw std::invalid_argument ──────────────────────────
    ISO_CHECK(throws_invalid([] { nf::parse_note(""); }));
    ISO_CHECK(throws_invalid([] { nf::parse_note("H4"); }));
    ISO_CHECK(throws_invalid([] { nf::parse_note("A"); }));
    ISO_CHECK(throws_invalid([] { nf::parse_note("A#"); }));
    ISO_CHECK(throws_invalid([] { nf::parse_note("Ax4"); }));
    ISO_CHECK(throws_invalid([] { nf::parse_note("A#b3"); }));
    ISO_CHECK(throws_invalid([] { nf::parse_note("Cb3"); }));

    // ── equality ──────────────────────────────────────────────────────────
    ISO_CHECK(nf::parse_note("A4") == nf::parse_note("A4"));
    ISO_CHECK(nf::parse_note("A4") != nf::parse_note("A5"));

    // ── octave bounds [-100, 100] ─────────────────────────────────────────
    ISO_CHECK(nf::validate_frequency(nf::note_to_frequency("A100")) > 0.0);
    ISO_CHECK(nf::validate_frequency(nf::note_to_frequency("A-100")) > 0.0);
    ISO_CHECK(nf::validate_frequency(nf::note_to_frequency("B100")) > 0.0);
    ISO_CHECK(nf::validate_frequency(nf::note_to_frequency("C-100")) > 0.0);
    ISO_CHECK(throws_invalid([] { nf::note_to_frequency("A101"); }));
    ISO_CHECK(throws_invalid([] { nf::note_to_frequency("A-101"); }));
    ISO_CHECK(throws_invalid([] { nf::note_to_frequency("A200000000"); }));
    ISO_CHECK(throws_invalid([] { nf::note_to_frequency("A-200000000"); }));
}

void test_hostile_inputs() {
    // CWE-476: nullptr on const char* overloads (Note constructors & free functions)
    ISO_CHECK(throws_invalid([] {
        const char* p = nullptr;
        nf::Note(p, "", 4);
    }));
    ISO_CHECK(throws_invalid([] {
        const char* p = nullptr;
        nf::Note("A", p, 4);
    }));
    ISO_CHECK(throws_invalid([] {
        const char* p = nullptr;
        nf::Note(p, std::string(""), 4);
    }));
    ISO_CHECK(throws_invalid([] {
        const char* p = nullptr;
        nf::Note(std::string("A"), p, 4);
    }));
    ISO_CHECK(throws_invalid([] {
        const char* p = nullptr;
        nf::Note(p, p, 4);
    }));
    ISO_CHECK(throws_invalid([] {
        const char* p = nullptr;
        nf::parse_note(p);
    }));
    ISO_CHECK(throws_invalid([] {
        const char* p = nullptr;
        nf::note_to_frequency(p);
    }));
    ISO_CHECK(throws_invalid([] {
        const char* p = nullptr;
        nf::note_to_frequency_with_reference(p, 440.0);
    }));

    // CWE-20: reject multi-character letters (no silent truncation)
    ISO_CHECK(throws_invalid([] { nf::Note("Cab", "", 4); }));
    ISO_CHECK(throws_invalid([] { nf::Note("C#", "", 4); }));
    ISO_CHECK(throws_invalid([] { nf::Note("AA", "", 4); }));
    ISO_CHECK(throws_invalid([] { nf::Note("C ", "", 4); }));
    ISO_CHECK(throws_invalid([] { nf::Note("", "", 4); }));

    // CWE-20: reject multi-character / invalid accidentals
    ISO_CHECK(throws_invalid([] { nf::Note("C", "##", 4); }));
    ISO_CHECK(throws_invalid([] { nf::Note("C", "#b", 4); }));
    ISO_CHECK(throws_invalid([] { nf::Note("C", "bb", 4); }));
    ISO_CHECK(throws_invalid([] { nf::Note("C", "# ", 4); }));
    ISO_CHECK(throws_invalid([] { nf::Note("C", "x", 4); }));
    ISO_CHECK(throws_invalid([] { nf::Note("C", std::string(1, '\0'), 4); }));
    ISO_CHECK(throws_invalid([] { nf::parse_note(std::string("A4\0x", 4)); }));

    // CWE-20 / CWE-190: octave bounds including INT_MIN / INT_MAX
    ISO_CHECK(throws_invalid([] { nf::Note("A", "", INT_MIN); }));
    ISO_CHECK(throws_invalid([] { nf::Note("A", "", INT_MAX); }));
    ISO_CHECK(throws_invalid([] { nf::Note("A", "", -101); }));
    ISO_CHECK(throws_invalid([] { nf::Note("A", "", 101); }));
    ISO_CHECK_EQ_INT(nf::Note("A", "", -100).octave(), -100);
    ISO_CHECK_EQ_INT(nf::Note("A", "", 100).octave(), 100);

    // CWE-190: pow2i(INT_MIN) & pow2i(INT_MAX) UB-free
    ISO_CHECK_EQ_DBL(nf::detail::pow2i(INT_MIN), 0.0, 0.0);
    ISO_CHECK(nf::detail::pow2i(INT_MAX) > 1e300);

    // CWE-400: MAX_NOTE_STRING_LENGTH (32) enforcement
    {
        std::string exact32 = "A" + std::string(30, '0') + "4";
        std::string over33 = "A" + std::string(31, '0') + "4";
        std::string huge = "A" + std::string(4096, '4');

        ISO_CHECK_EQ_INT(nf::parse_note(exact32).octave(), 4);
        ISO_CHECK_EQ_INT(nf::parse_note(exact32.c_str()).octave(), 4);
        ISO_CHECK(throws_invalid([&] { nf::parse_note(over33); }));
        ISO_CHECK(throws_invalid([&] { nf::parse_note(over33.c_str()); }));
        ISO_CHECK(throws_invalid([&] { nf::parse_note(huge); }));
        ISO_CHECK(throws_invalid([&] { nf::parse_note(huge.c_str()); }));
        ISO_CHECK(throws_invalid([&] { nf::Note(huge, "", 4); }));
        ISO_CHECK(throws_invalid([&] { nf::Note(huge.c_str(), "", 4); }));
        ISO_CHECK(throws_invalid([&] { nf::Note(huge.c_str(), std::string(""), 4); }));
        ISO_CHECK(throws_invalid([&] { nf::Note("A", huge, 4); }));
        ISO_CHECK(throws_invalid([&] { nf::Note("A", huge.c_str(), 4); }));
        ISO_CHECK(throws_invalid([&] { nf::Note(std::string("A"), huge.c_str(), 4); }));
        ISO_CHECK(throws_invalid([&] { nf::Note(huge.c_str(), huge.c_str(), 4); }));
    }

    // CWE-20: trailing garbage, whitespace, signs, extreme digits
    {
        const char* bad_notes[] = {
            "A4 ", " A4", "A4\n", "A4\t", "A4x", "A4#", "A+4", "A--4",
            "A-", "#4", "4A", "A##4", "A-2147483648", "A2147483647",
            "A99999999999999999999999999999", "Cb-100", "E#100"
        };
        for (const char* s : bad_notes) {
            ISO_CHECK(throws_invalid([&] { nf::parse_note(s); }));
            ISO_CHECK(throws_invalid([&] { nf::parse_note(std::string(s)); }));
        }
    }

    // CWE-20: non-finite & out-of-range frequencies
    {
        const double nan_v = std::numeric_limits<double>::quiet_NaN();
        const double inf_v = std::numeric_limits<double>::infinity();
        const double ninf_v = -std::numeric_limits<double>::infinity();

        ISO_CHECK_EQ_DBL(nf::validate_frequency(440.0), 440.0, 1e-12);
        ISO_CHECK(throws_invalid([&] { nf::validate_frequency(nan_v); }));
        ISO_CHECK(throws_invalid([&] { nf::validate_frequency(inf_v); }));
        ISO_CHECK(throws_invalid([&] { nf::validate_frequency(ninf_v); }));
        ISO_CHECK(throws_invalid([] { nf::validate_frequency(0.0); }));
        ISO_CHECK(throws_invalid([] { nf::validate_frequency(-0.0); }));
        ISO_CHECK(throws_invalid([] { nf::validate_frequency(-440.0); }));
        ISO_CHECK(throws_invalid([] { nf::validate_frequency(1e39); }));
        ISO_CHECK(throws_invalid([] { nf::validate_frequency(1e308); }));

        ISO_CHECK_EQ_DBL(nf::note_to_frequency_with_reference("A5", 432.0),
                         864.0, 1e-6);
        ISO_CHECK(throws_invalid([&] {
            nf::note_to_frequency_with_reference("A4", nan_v);
        }));
        ISO_CHECK(throws_invalid([&] {
            nf::note_to_frequency_with_reference("A4", inf_v);
        }));
        ISO_CHECK(throws_invalid([&] {
            nf::note_to_frequency_with_reference("A4", ninf_v);
        }));
        ISO_CHECK(throws_invalid([] {
            nf::note_to_frequency_with_reference("A4", 0.0);
        }));
        ISO_CHECK(throws_invalid([] {
            nf::note_to_frequency_with_reference("A4", -440.0);
        }));
        ISO_CHECK(throws_invalid([] {
            nf::note_to_frequency_with_reference("A4", 1e7);
        }));
    }
}

void test_pbt_splitmix64() {
    struct Spelling {
        char letter;
        const char* acc;
    };
    static const Spelling spellings[17] = {
        {'C', ""}, {'C', "#"}, {'D', "b"}, {'D', ""}, {'D', "#"}, {'E', "b"},
        {'E', ""}, {'F', ""},  {'F', "#"}, {'G', "b"}, {'G', ""}, {'G', "#"},
        {'A', "b"}, {'A', ""}, {'A', "#"}, {'B', "b"}, {'B', ""}
    };

    std::uint64_t rng = UINT64_C(0x123456789abcdef0);
    for (int iter = 0; iter < 500; ++iter) {
        std::uint64_t r1 = splitmix64(rng);
        std::uint64_t r2 = splitmix64(rng);
        std::uint64_t r3 = splitmix64(rng);
        std::size_t sp_idx = static_cast<std::size_t>(r1 % 17u);
        int oct = static_cast<int>(r2 % 201u) - 100;  // [-100, 100]
        bool lower = (r3 & 1u) != 0;

        char ch = lower ? static_cast<char>(spellings[sp_idx].letter - 'A' + 'a')
                        : spellings[sp_idx].letter;
        nf::Note n(std::string(1, ch), spellings[sp_idx].acc, oct);
        nf::Note parsed = nf::parse_note(n.to_string());
        ISO_CHECK(parsed == n);
        ISO_CHECK_EQ_INT(parsed.semitones_from_a4(), n.semitones_from_a4());

        double f = n.frequency();
        ISO_CHECK_EQ_DBL(nf::validate_frequency(f), f, 0.0);

        if (oct < nf::MAX_OCTAVE) {
            nf::Note up(std::string(1, ch), spellings[sp_idx].acc, oct + 1);
            ISO_CHECK_EQ_INT(up.semitones_from_a4() - n.semitones_from_a4(), 12);
            ISO_CHECK(approx_rel(up.frequency() / f, 2.0, 1e-6));
        }

        double ref_hz = 20.0 + static_cast<double>(r3 % 19980u);
        double f_ref = n.frequency_with_reference(ref_hz);
        ISO_CHECK(approx_rel(f_ref / f, ref_hz / 440.0, 1e-6));
    }

    // Fuzz random ASCII strings via SplitMix64
    for (int iter = 0; iter < 300; ++iter) {
        std::size_t len = static_cast<std::size_t>(splitmix64(rng) % 64u);
        std::string fuzz;
        fuzz.reserve(len);
        for (std::size_t j = 0; j < len; ++j) {
            char ch = static_cast<char>(1u + (splitmix64(rng) % 127u));
            fuzz.push_back(ch);
        }

        try {
            nf::Note out = nf::parse_note(fuzz);
            ISO_CHECK(len >= 2 && len <= nf::MAX_NOTE_STRING_LENGTH);
            ISO_CHECK(out.octave() >= nf::MIN_OCTAVE &&
                      out.octave() <= nf::MAX_OCTAVE);
            ISO_CHECK(out.chromatic_index() >= 0);
            ISO_CHECK(nf::validate_frequency(out.frequency()) > 0.0);
        } catch (const std::invalid_argument&) {
            // Expected fail-closed rejection for malformed/oversized strings
        }
    }
}

}  // namespace

int main() {
    test_reference_and_core();
    test_hostile_inputs();
    test_pbt_splitmix64();
    return ISO_TEST_RESULT();
}
