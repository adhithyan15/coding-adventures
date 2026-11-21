/*
 * note_frequency.h — musical note names to pitch frequencies, in pure ISO C17.
 * A faithful port of the Rust `note-frequency` crate.
 * ===========================================================================
 *
 * Western music divides each octave into twelve equal semitones (12-tone equal
 * temperament). Fixing concert A4 = 440 Hz, any note's frequency is:
 *
 *     freq = 440 * 2^(semitones_from_A4 / 12)
 *
 * A note is a letter (A-G), an optional accidental (# or b), and an octave
 * number, e.g. "A4", "C#5", "Db3". Only natural notes plus a single sharp or
 * flat are supported, and only the spellings that name a real chromatic pitch
 * (so "Cb" / "E#" / "B#" / "Fb" are rejected, matching the Rust crate).
 *
 * NO libm: the one power (2^x) is computed from a from-scratch e^x. Frequencies
 * match the Rust `f64::powf` results to well within 1e-6.
 *
 * VER15 HARDENING:
 * - CWE-400: Input note strings are bounded to NF_MAX_NOTE_STRING_LENGTH (32)
 *   before parsing.
 * - CWE-20 / CWE-190: Octaves are bounded to [NF_MIN_OCTAVE, NF_MAX_OCTAVE]
 *   ([-100, 100]), multi-character letters are rejected without truncation,
 *   64-bit integer semitone arithmetic avoids overflow, pow2i handles INT_MIN
 *   without signed negation UB, and frequency inputs/outputs are validated to
 *   be finite and within range.
 * - CWE-476: All public functions guard against NULL pointers.
 *
 * PORTABILITY. Pure ISO C17, no <math.h>, no compiler extensions. Builds clean
 * under GCC, Clang, and MSVC with -pedantic-errors / /permissive- and
 * warnings-as-errors.
 */
#ifndef CA_NOTE_FREQUENCY_H
#define CA_NOTE_FREQUENCY_H

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

#define NF_MAX_NOTE_STRING_LENGTH 32
#define NF_MIN_OCTAVE (-100)
#define NF_MAX_OCTAVE 100
#define NF_MAX_FREQUENCY_HZ 1e38
#define NF_MAX_REFERENCE_FREQUENCY_HZ 1e6

/* Status of a fallible operation. */
typedef enum {
    NF_OK = 0,
    NF_ERR_INVALID_SPELLING,  /* letter+accidental is not a real chromatic pitch */
    NF_ERR_INVALID_NOTE,      /* text is malformed or octave is out of bounds */
    NF_ERR_NULL_POINTER,      /* NULL pointer passed to API */
    NF_ERR_INVALID_FREQUENCY  /* non-finite or out-of-range frequency */
} NfStatus;

/* A parsed note: an uppercase letter A-G, an accidental ("", "#", or "b"), and
 * an octave. A plain value — copy it freely, no cleanup required. */
typedef struct {
    char letter;        /* 'A'..'G' (uppercase) */
    char accidental[3]; /* "", "#", or "b" (NUL-terminated) */
    int octave;
} NfNote;

/* Build a note from its parts. `letter` must be a single character A-G (lower-
 * or uppercase); `accidental` must be "", "#", or "b"; `octave` must be in
 * [NF_MIN_OCTAVE, NF_MAX_OCTAVE]. Writes *out on NF_OK. */
NfStatus nf_note_new(const char *letter, const char *accidental, int octave,
                     NfNote *out);

/* Write the spelling (letter + accidental, e.g. "C#") into buf, NUL-terminated.
 * buf should hold at least 3 bytes. Writes "" if n is NULL/invalid (when
 * buf != NULL && bufsz > 0). */
void nf_note_spelling(const NfNote *n, char *buf, size_t bufsz);

/* The note's position within the octave, 0 (C) .. 11 (B), or -1 if n is NULL
 * or has an invalid spelling. */
int nf_note_chromatic_index(const NfNote *n);

/* Signed semitone distance from A4 (A4 == 0), or 0 if n is NULL/invalid. */
int nf_note_semitones_from_a4(const NfNote *n);

/* Checked signed semitone distance from A4 via *out. */
NfStatus nf_note_checked_semitones_from_a4(const NfNote *n, int *out);

/* The note's frequency in Hz (A4 == 440), or 0.0 if n is NULL/invalid. */
double nf_note_frequency(const NfNote *n);

/* Checked note frequency in Hz (A4 == 440) via *out. */
NfStatus nf_note_checked_frequency(const NfNote *n, double *out);

/* Validate that a frequency is finite, strictly positive, and <= NF_MAX_FREQUENCY_HZ. */
NfStatus nf_validate_frequency(double freq_hz);

/* Compute the note's frequency in Hz using a custom A4 reference frequency. */
NfStatus nf_note_frequency_with_reference(const NfNote *n, double a4_hz,
                                          double *out);

/* Write "<spelling><octave>" (e.g. "C#5") into buf, NUL-terminated. buf should
 * hold at least 16 bytes to be safe for any octave. Writes "" if n is
 * NULL/invalid (when buf != NULL && bufsz > 0). */
void nf_note_to_string(const NfNote *n, char *buf, size_t bufsz);

/* Parse "<letter><optional # or b><octave>" (e.g. "A4", "C#5", "Db3"). Writes
 * *out on NF_OK, or an NF_ERR_* status on failure. */
NfStatus nf_parse_note(const char *text, NfNote *out);

/* Parse a note and return its frequency in Hz via *out. */
NfStatus nf_note_to_frequency(const char *text, double *out);

/* Parse a note and return its frequency in Hz via *out using a custom A4 reference. */
NfStatus nf_note_to_frequency_with_reference(const char *text, double a4_hz,
                                             double *out);

#ifdef __cplusplus
}
#endif

#endif /* CA_NOTE_FREQUENCY_H */
