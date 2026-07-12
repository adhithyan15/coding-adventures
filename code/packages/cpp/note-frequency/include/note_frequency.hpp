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
// DIVERGENCE FROM RUST. Rust returns `Result<_, String>`; this port throws
// std::invalid_argument on a bad note or spelling.
//
// PORTABILITY. Pure ISO C++17, no <cmath>, no compiler extensions.
#ifndef CA_NOTE_FREQUENCY_HPP
#define CA_NOTE_FREQUENCY_HPP

#include <cstddef>
#include <stdexcept>
#include <string>

namespace ca {
namespace note_frequency {

namespace detail {

inline constexpr int REFERENCE_OCTAVE = 4;
inline constexpr int REFERENCE_INDEX = 9;
inline constexpr double REFERENCE_FREQUENCY_HZ = 440.0;
inline constexpr int SEMITONES_PER_OCTAVE = 12;

inline double pow2i(int k) {
    double result = 1.0;
    double base = k < 0 ? 0.5 : 2.0;
    int n = k < 0 ? -k : k;
    while (n > 0) {
        if (n & 1) result *= base;
        base *= base;
        n >>= 1;
    }
    return result;
}

// e^x via Cody-Waite range reduction (x = k*ln2 + r, |r| <= ln2/2).
inline double d_exp(double x) {
    if (x != x) return x;  // NaN propagates (and stays out of the int cast)
    if (x == 0.0) return 1.0;
    // Bound |x| before the (int) range reduction: an extreme octave can drive
    // the exponent past these limits, and casting a huge double to int is UB.
    if (x > 709.782712893384) return 1.7976931348623157e308;
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
inline int chromatic_index_for(const std::string& s) {
    struct Entry {
        const char* spelling;
        int idx;
    };
    static const Entry table[] = {
        {"C", 0},  {"C#", 1}, {"Db", 1},  {"D", 2},   {"D#", 3}, {"Eb", 3},
        {"E", 4},  {"F", 5},  {"F#", 6},  {"Gb", 6},  {"G", 7},  {"G#", 8},
        {"Ab", 8}, {"A", 9},  {"A#", 10}, {"Bb", 10}, {"B", 11},
    };
    for (const Entry& e : table)
        if (s == e.spelling) return e.idx;
    return -1;
}

inline char upper_ascii(char c) {
    return (c >= 'a' && c <= 'z') ? static_cast<char>(c - 'a' + 'A') : c;
}

}  // namespace detail

// A parsed note: an uppercase letter A-G, an accidental ("", "#", "b"), octave.
class Note {
public:
    // Build from parts; `letter` may be any case, `accidental` must be "",
    // "#", or "b". Throws std::invalid_argument on an unsupported spelling.
    Note(const std::string& letter, const std::string& accidental, int octave)
        : letter_(1, detail::upper_ascii(letter.empty() ? '?' : letter[0])),
          accidental_(accidental),
          octave_(octave) {
        if (detail::chromatic_index_for(spelling()) < 0) {
            throw std::invalid_argument(
                "Unsupported note spelling \"" + spelling() +
                "\". Only natural notes plus single # or b are supported.");
        }
    }

    const std::string& letter() const { return letter_; }
    const std::string& accidental() const { return accidental_; }
    int octave() const { return octave_; }

    std::string spelling() const { return letter_ + accidental_; }

    int chromatic_index() const {
        return detail::chromatic_index_for(spelling());
    }

    int semitones_from_a4() const {
        // Exact for any sensible octave; the narrowing cast for a pathological
        // octave is implementation-defined, never undefined behavior.
        return static_cast<int>(semitones_ll());
    }

    double frequency() const {
        // Use the wide value so the frequency is well-defined even for extreme
        // octaves (they simply saturate to 0 or +inf).
        double exponent = static_cast<double>(semitones_ll()) /
                          static_cast<double>(detail::SEMITONES_PER_OCTAVE);
        return detail::REFERENCE_FREQUENCY_HZ * detail::exp2_d(exponent);
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
    // Semitone distance in a wide type: octave may be any int, so
    // `(octave - 4) * 12` is computed in long long to avoid signed-overflow UB.
    long long semitones_ll() const {
        long long octave_offset =
            (static_cast<long long>(octave_) - detail::REFERENCE_OCTAVE) *
            detail::SEMITONES_PER_OCTAVE;
        long long pitch_offset =
            static_cast<long long>(chromatic_index()) - detail::REFERENCE_INDEX;
        return octave_offset + pitch_offset;
    }

    std::string letter_;
    std::string accidental_;
    int octave_;
};

namespace detail {

inline bool is_canonical_octave(const std::string& text) {
    std::size_t start = 0;
    if (!text.empty() && text[0] == '-') start = 1;
    if (start >= text.size()) return false;  // empty or just "-"
    for (std::size_t i = start; i < text.size(); i++)
        if (text[i] < '0' || text[i] > '9') return false;
    return true;
}

}  // namespace detail

// Parse "<letter><optional # or b><octave>" (e.g. "A4", "C#5", "Db3"). Throws
// std::invalid_argument on a malformed note or unsupported spelling.
inline Note parse_note(const std::string& text) {
    auto invalid = [&]() {
        return std::invalid_argument(
            "Invalid note \"" + text +
            "\". Expected <letter><optional # or b><octave>, e.g. 'A4'.");
    };
    if (text.empty()) throw invalid();
    char letter = text[0];
    char up = detail::upper_ascii(letter);
    if (up < 'A' || up > 'G') throw invalid();

    std::string rest = text.substr(1);
    std::string accidental;
    std::string octave_text;
    if (!rest.empty() && rest[0] == '#') {
        accidental = "#";
        octave_text = rest.substr(1);
    } else if (!rest.empty() && rest[0] == 'b') {
        accidental = "b";
        octave_text = rest.substr(1);
    } else {
        octave_text = rest;
    }

    if (octave_text.empty() || !detail::is_canonical_octave(octave_text))
        throw invalid();

    int octave = 0;
    try {
        std::size_t pos = 0;
        long v = std::stol(octave_text, &pos);
        if (pos != octave_text.size() || v < -2147483647L - 1 ||
            v > 2147483647L)
            throw invalid();
        octave = static_cast<int>(v);
    } catch (const std::out_of_range&) {
        throw invalid();
    } catch (const std::invalid_argument&) {
        throw invalid();
    }

    return Note(std::string(1, letter), accidental, octave);
}

// Parse a note and return its frequency in Hz.
inline double note_to_frequency(const std::string& text) {
    return parse_note(text).frequency();
}

}  // namespace note_frequency
}  // namespace ca

#endif  // CA_NOTE_FREQUENCY_HPP
