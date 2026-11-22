// note_frequency.hpp — musical note names to pitch frequencies, header-only in
// pure ISO C++17 (namespace ca::note_frequency). A faithful port of the Rust
// `note-frequency` crate.
// ===========================================================================
//
// Western music divides each octave into twelve equal semitones (12-tone equal
// temperament). Fixing concert A4 = 440 Hz, a note's frequency is:
//
//     freq = 440 * 2^(semitones_from_A4 / 12)
//
// A note is a letter (A-G), an optional accidental (# or b), and an octave, e.g.
// "A4", "C#5", "Db3". Only natural notes plus a single sharp or flat, and only
// spellings that name a real chromatic pitch (so "Cb"/"E#"/"B#"/"Fb" are
// rejected, matching the Rust crate).
//
// NO libm / <cmath>: the one power (2^x) is computed from a from-scratch e^x.
//
// VER15 HARDENING:
// - CWE-400: Input note strings are bounded to MAX_NOTE_STRING_LENGTH (32)
//   before parsing or allocating std::string.
// - CWE-20 / CWE-190: Octaves are bounded to [MIN_OCTAVE, MAX_OCTAVE]
//   ([-100, 100]), multi-character letters are rejected without truncation,
//   std::int64_t semitone arithmetic avoids overflow, pow2i handles INT_MIN
//   without signed negation UB, and frequencies are validated to be finite and
//   within range.
// - CWE-476: const char* entry points reject nullptr safely.
//
// DIVERGENCE FROM RUST. Rust returns `Result<_, String>`; this port throws
// std::invalid_argument on a bad note, spelling, octave, or frequency.
//
// PORTABILITY. Pure ISO C++17, no <cmath>, no compiler extensions.
#ifndef CA_NOTE_FREQUENCY_HPP
#define CA_NOTE_FREQUENCY_HPP

#include <cstddef>
#include <cstdint>
#include <limits>
#include <stdexcept>
#include <string>

namespace ca {
namespace note_frequency {

inline constexpr std::size_t MAX_NOTE_STRING_LENGTH = 32;
inline constexpr int MIN_OCTAVE = -100;
inline constexpr int MAX_OCTAVE = 100;
inline constexpr double MAX_FREQUENCY_HZ = 1e38;
inline constexpr double MAX_REFERENCE_FREQUENCY_HZ = 1e6;

namespace detail {

inline constexpr int REFERENCE_OCTAVE = 4;
inline constexpr int REFERENCE_INDEX = 9;
inline constexpr double REFERENCE_FREQUENCY_HZ = 440.0;
inline constexpr int SEMITONES_PER_OCTAVE = 12;

inline bool is_finite(double x) {
    return (x == x) && ((x - x) == 0.0) &&
           (x <= std::numeric_limits<double>::max()) &&
           (x >= -std::numeric_limits<double>::max());
}

inline double pow2i(int k) {
    double result = 1.0;
    double base = k < 0 ? 0.5 : 2.0;
    // Convert magnitude in unsigned int so k == INT_MIN never triggers UB.
    unsigned int n = k < 0 ? static_cast<unsigned int>(-(k + 1)) + 1u
                           : static_cast<unsigned int>(k);
    while (n > 0u) {
        if (n & 1u) result *= base;
        n >>= 1;
        if (n > 0u) base *= base;
    }
    return result;
}

// e^x via Cody-Waite range reduction (x = k*ln2 + r, |r| <= ln2/2).
inline double d_exp(double x) {
    if (x != x) return x;  // NaN propagates (and stays out of the int cast)
    if (x == 0.0) return 1.0;
    // Bound |x| before the (int) range reduction: an extreme exponent can
    // overflow double/int; e^x overflows above ~709.78 and underflows below
    // ~-745.13 anyway.
    if (x > 709.782712893384) return std::numeric_limits<double>::max();
    if (x < -745.13321910194) return 0.0;
    constexpr double INV_LN2 = 1.4426950408889634;
    constexpr double C1 = 0.693359375;
    constexpr double C2 = -2.1219444005469058277e-4;
    double kf = x * INV_LN2;
    int k = static_cast<int>(kf >= 0.0 ? kf + 0.5 : kf - 0.5);
    double r = (x - static_cast<double>(k) * C1) - static_cast<double>(k) * C2;
    double term = 1.0, sum = 1.0;
    for (int i = 1; i <= 17; i++) {
        term *= r / static_cast<double>(i);
        sum += term;
    }
    return sum * pow2i(k);
}

inline double exp2_d(double y) {
    constexpr double LN2 = 0.6931471805599453;
    return d_exp(y * LN2);
}

// 0..11 chromatic index for a spelling, or -1 if it names no real pitch.
inline int chromatic_index_for(const char* spelling) {
    struct Entry {
        const char* spelling;
        int idx;
    };
    static const Entry table[] = {
        {"C", 0},  {"C#", 1}, {"Db", 1},  {"D", 2},   {"D#", 3}, {"Eb", 3},
        {"E", 4},  {"F", 5},  {"F#", 6},  {"Gb", 6},  {"G", 7},  {"G#", 8},
        {"Ab", 8}, {"A", 9},  {"A#", 10}, {"Bb", 10}, {"B", 11},
    };
    for (const Entry& e : table) {
        const char* a = spelling;
        const char* b = e.spelling;
        while (*a != '\0' && *a == *b) {
            ++a;
            ++b;
        }
        if (*a == '\0' && *b == '\0') return e.idx;
    }
    return -1;
}

inline int chromatic_index_parts(char letter, const std::string& accidental) {
    char buf[4];
    if (accidental.empty()) {
        buf[0] = letter;
        buf[1] = '\0';
    } else if (accidental.size() == 1 &&
               (accidental[0] == '#' || accidental[0] == 'b')) {
        buf[0] = letter;
        buf[1] = accidental[0];
        buf[2] = '\0';
    } else {
        return -1;
    }
    return chromatic_index_for(buf);
}

inline char upper_ascii(char c) {
    return (c >= 'a' && c <= 'z') ? static_cast<char>(c - 'a' + 'A') : c;
}

inline std::size_t bounded_strlen(const char* s, std::size_t max_len) {
    std::size_t len = 0;
    while (len <= max_len && s[len] != '\0') {
        ++len;
    }
    return len;
}

inline std::string checked_letter_cstr(const char* letter) {
    if (letter == nullptr) {
        throw std::invalid_argument(
            "Unsupported note letter: null pointer.");
    }
    if (bounded_strlen(letter, 1) != 1) {
        throw std::invalid_argument(
            "Unsupported note letter: must be a single character A-G.");
    }
    return std::string(1, letter[0]);
}

inline std::string checked_accidental_cstr(const char* accidental) {
    if (accidental == nullptr) {
        throw std::invalid_argument(
            "Unsupported note accidental: null pointer.");
    }
    std::size_t len = bounded_strlen(accidental, 1);
    if (len > 1) {
        throw std::invalid_argument(
            "Unsupported note accidental: must be \"\", \"#\", or \"b\".");
    }
    return std::string(accidental, len);
}

inline bool parse_octave(const std::string& text, int& out) {
    if (text.empty()) return false;
    std::size_t i = 0;
    bool neg = false;
    if (text[0] == '-') {
        neg = true;
        i = 1;
    }
    if (i >= text.size()) return false;
    int val = 0;
    for (; i < text.size(); ++i) {
        char c = text[i];
        if (c < '0' || c > '9') return false;
        val = val * 10 + (c - '0');
        // Since |MIN_OCTAVE| == MAX_OCTAVE == 100, exceeding 100 guarantees out
        // of range and prevents signed int overflow.
        if (val > MAX_OCTAVE) return false;
    }
    if (neg) val = -val;
    if (val < MIN_OCTAVE || val > MAX_OCTAVE) return false;
    out = val;
    return true;
}

}  // namespace detail

// Validate that a frequency in Hz is finite, strictly positive, and <= MAX_FREQUENCY_HZ.
inline double validate_frequency(double freq_hz) {
    if (!detail::is_finite(freq_hz) || freq_hz <= 0.0 ||
        freq_hz > MAX_FREQUENCY_HZ) {
        throw std::invalid_argument(
            "Invalid frequency: must be finite, > 0.0, and within range.");
    }
    return freq_hz;
}

// A parsed note: an uppercase letter A-G, an accidental ("", "#", "b"), octave.
class Note {
public:
    // Build from parts; `letter` must be a single character A-G (any case),
    // `accidental` must be "", "#", or "b", `octave` in [MIN_OCTAVE, MAX_OCTAVE].
    // Throws std::invalid_argument on invalid input or unsupported spelling.
    Note(const std::string& letter, const std::string& accidental, int octave)
        : letter_(), accidental_(), octave_(octave) {
        if (letter.size() != 1) {
            throw std::invalid_argument(
                "Unsupported note letter: must be a single character A-G.");
        }
        if (accidental.size() > 1) {
            throw std::invalid_argument(
                "Unsupported note accidental: must be \"\", \"#\", or \"b\".");
        }
        if (octave < MIN_OCTAVE || octave > MAX_OCTAVE) {
            throw std::invalid_argument(
                "Octave out of bounds [-100, 100].");
        }
        char canonical = detail::upper_ascii(letter[0]);
        if (detail::chromatic_index_parts(canonical, accidental) < 0) {
            throw std::invalid_argument(
                std::string("Unsupported note spelling \"") + canonical +
                accidental +
                "\". Only natural notes plus single # or b are supported.");
        }
        letter_.assign(1, canonical);
        accidental_ = accidental;
    }

    Note(const char* letter, const char* accidental, int octave)
        : Note(detail::checked_letter_cstr(letter),
               detail::checked_accidental_cstr(accidental), octave) {}

    Note(const char* letter, const std::string& accidental, int octave)
        : Note(detail::checked_letter_cstr(letter), accidental, octave) {}

    Note(const std::string& letter, const char* accidental, int octave)
        : Note(letter, detail::checked_accidental_cstr(accidental), octave) {}

    const std::string& letter() const { return letter_; }
    const std::string& accidental() const { return accidental_; }
    int octave() const { return octave_; }

    std::string spelling() const { return letter_ + accidental_; }

    int chromatic_index() const {
        return detail::chromatic_index_parts(letter_[0], accidental_);
    }

    int semitones_from_a4() const {
        return static_cast<int>(semitones_i64());
    }

    double frequency_with_reference(double a4_hz) const {
        if (!detail::is_finite(a4_hz) || a4_hz <= 0.0 ||
            a4_hz > MAX_REFERENCE_FREQUENCY_HZ) {
            throw std::invalid_argument(
                "Invalid reference frequency: must be finite and in (0, 1e6].");
        }
        double exponent = static_cast<double>(semitones_i64()) /
                          static_cast<double>(detail::SEMITONES_PER_OCTAVE);
        return validate_frequency(a4_hz * detail::exp2_d(exponent));
    }

    double frequency() const {
        return frequency_with_reference(detail::REFERENCE_FREQUENCY_HZ);
    }

    std::string to_string() const {
        return spelling() + std::to_string(octave_);
    }

    bool operator==(const Note& o) const {
        return letter_ == o.letter_ && accidental_ == o.accidental_ &&
               octave_ == o.octave_;
    }
    bool operator!=(const Note& o) const { return !(*this == o); }

private:
    // Semitone distance in std::int64_t to prevent signed overflow.
    std::int64_t semitones_i64() const {
        std::int64_t octave_offset =
            (static_cast<std::int64_t>(octave_) - detail::REFERENCE_OCTAVE) *
            detail::SEMITONES_PER_OCTAVE;
        std::int64_t pitch_offset =
            static_cast<std::int64_t>(chromatic_index()) -
            detail::REFERENCE_INDEX;
        return octave_offset + pitch_offset;
    }

    std::string letter_;
    std::string accidental_;
    int octave_;
};

// Parse "<letter><optional # or b><octave>" (e.g. "A4", "C#5", "Db3"). Throws
// std::invalid_argument on a malformed note, oversized input, or unsupported
// spelling.
inline Note parse_note(const std::string& text) {
    if (text.empty() || text.size() > MAX_NOTE_STRING_LENGTH) {
        throw std::invalid_argument(
            "Invalid note string: length must be in [1, 32].");
    }
    auto invalid = [&]() {
        return std::invalid_argument(
            "Invalid note \"" + text +
            "\". Expected <letter><optional # or b><octave>, e.g. 'A4'.");
    };
    char letter = text[0];
    char up = detail::upper_ascii(letter);
    if (up < 'A' || up > 'G') throw invalid();

    std::size_t idx = 1;
    std::string accidental;
    if (idx < text.size() && text[idx] == '#') {
        accidental = "#";
        ++idx;
    } else if (idx < text.size() && text[idx] == 'b') {
        accidental = "b";
        ++idx;
    }

    std::string octave_text = text.substr(idx);
    int octave = 0;
    if (!detail::parse_octave(octave_text, octave)) {
        throw invalid();
    }

    return Note(std::string(1, letter), accidental, octave);
}

inline Note parse_note(const char* text) {
    if (text == nullptr) {
        throw std::invalid_argument("Invalid note: null pointer.");
    }
    // Enforce MAX_NOTE_STRING_LENGTH before allocating std::string (CWE-400).
    std::size_t len = detail::bounded_strlen(text, MAX_NOTE_STRING_LENGTH);
    if (len == 0 || len > MAX_NOTE_STRING_LENGTH) {
        throw std::invalid_argument(
            "Invalid note string: length must be in [1, 32].");
    }
    return parse_note(std::string(text, len));
}

// Parse a note and return its frequency in Hz.
inline double note_to_frequency(const std::string& text) {
    return parse_note(text).frequency();
}

inline double note_to_frequency(const char* text) {
    return parse_note(text).frequency();
}

// Parse a note and return its frequency in Hz using a custom A4 reference.
inline double note_to_frequency_with_reference(const std::string& text,
                                               double a4_hz) {
    return parse_note(text).frequency_with_reference(a4_hz);
}

inline double note_to_frequency_with_reference(const char* text,
                                               double a4_hz) {
    return parse_note(text).frequency_with_reference(a4_hz);
}

}  // namespace note_frequency
}  // namespace ca

#endif  // CA_NOTE_FREQUENCY_HPP
