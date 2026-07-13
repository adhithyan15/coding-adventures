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
 * DIVERGENCE FROM RUST. Rust returns `Result<_, String>`; this port returns an
 * `NfStatus` code and writes results through out-parameters. `Note` is a small
 * fixed-size value (no allocation).
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

/* Status of a fallible operation. */
typedef enum {
    NF_OK = 0,
    NF_ERR_INVALID_SPELLING, /* letter+accidental is not a real chromatic pitch */
    NF_ERR_INVALID_NOTE      /* text is not <letter><# or b?><octave> */
} NfStatus;

/* A parsed note: an uppercase letter A-G, an accidental ("", "#", or "b"), and
 * an octave. A plain value — copy it freely, no cleanup required. */
typedef struct {
    char letter;        /* 'A'..'G' (uppercase) */
    char accidental[3]; /* "", "#", or "b" (NUL-terminated) */
    int octave;
} NfNote;

/* Build a note from its parts. `letter` may be lower- or uppercase; `accidental`
 * must be "", "#", or "b". Writes *out on NF_OK, or NF_ERR_INVALID_SPELLING when
 * letter+accidental is not a supported chromatic spelling. */
NfStatus nf_note_new(const char *letter, const char *accidental, int octave,
                     NfNote *out);

/* Write the spelling (letter + accidental, e.g. "C#") into buf, NUL-terminated.
 * buf should hold at least 3 bytes. */
void nf_note_spelling(const NfNote *n, char *buf, size_t bufsz);

/* The note's position within the octave, 0 (C) .. 11 (B). */
int nf_note_chromatic_index(const NfNote *n);

/* Signed semitone distance from A4 (A4 == 0). */
int nf_note_semitones_from_a4(const NfNote *n);

/* The note's frequency in Hz (A4 == 440). */
double nf_note_frequency(const NfNote *n);

/* Write "<spelling><octave>" (e.g. "C#5") into buf, NUL-terminated. buf should
 * hold at least 16 bytes to be safe for any octave. */
void nf_note_to_string(const NfNote *n, char *buf, size_t bufsz);

/* Parse "<letter><optional # or b><octave>" (e.g. "A4", "C#5", "Db3"). Writes
 * *out on NF_OK, or NF_ERR_INVALID_NOTE / NF_ERR_INVALID_SPELLING. */
NfStatus nf_parse_note(const char *text, NfNote *out);

/* Parse a note and return its frequency in Hz via *out. */
NfStatus nf_note_to_frequency(const char *text, double *out);

#ifdef __cplusplus
}
#endif

#endif /* CA_NOTE_FREQUENCY_H */
