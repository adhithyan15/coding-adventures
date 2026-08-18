//! VLT-PM44 pure password-generation policy, entropy floor, and sampler.
//!
//! # What this crate is
//!
//! This crate answers three questions and refuses to answer any others:
//!
//! 1. **Is this policy allowed?** A length, a set of character classes, and an
//!    ambiguity choice either describe a password worth minting or they do not.
//! 2. **How much randomness does it need?** An exact byte count, computed
//!    before a single byte is asked for.
//! 3. **Given exactly those bytes, what is the password?** A deterministic,
//!    exactly uniform mapping from bytes to characters.
//!
//! It deliberately owns **no randomness of its own**. There is no `rand`, no
//! thread-local generator, no seeding, and no way to call anything in here and
//! get a password without handing it bytes first. That is the whole design
//! idea, and it buys two things that matter more than convenience:
//!
//! - The caller — the CLI — is forced to name its entropy source out loud, and
//!   the only source it has is the operating-system CSPRNG.
//! - Every security property in `VLT-PM44-cli-password-generate.md` §3 and §4
//!   is testable against fixed byte vectors. "This sampler is unbiased" becomes
//!   an assertion about specific bytes rather than a statistical hope.
//!
//! # The three ideas, in order
//!
//! ## Idea one: entropy is a multiplication, so the floor is a comparison
//!
//! A password drawn by picking `length` characters independently and uniformly
//! from an alphabet of `n` characters is one of exactly `n^length` equally
//! likely strings. Its strength — "entropy", the number of bits an attacker
//! must brute-force — is `log2(n^length)`, which is `length * log2(n)`.
//!
//! | Policy | `n` | `length` | `n^length` | bits |
//! |---|---:|---:|---|---:|
//! | all four classes, default | 89 | 24 | ~10^46 | 155.4 |
//! | all four classes | 89 | 13 | ~10^25 | 84.2 |
//! | all four classes | 89 | 12 | ~10^23 | 77.7 |
//! | digits only | 10 | 4 | 10^4 | 13.3 |
//!
//! The floor is 80 bits, so the third row is refused and the second is not.
//!
//! Notice what the check does *not* do: it never computes `log2`. Comparing
//! `length * log2(n)` against `80.0` means deciding a security boundary with
//! floating-point rounding, and the 52-character and 26-character rows of
//! §4.3's table land within 0.2 bits of the line. Instead
//! [`meets_minimum_entropy`] asks the equivalent integer question
//! `n^length >= 2^80` by multiplying and stopping early. Same answer, no
//! rounding, identical on every platform.
//!
//! ## Idea two: reducing a random number modulo `n` is biased, and it needn't be
//!
//! Suppose you have a fair six-sided die and want a fair coin. "Odd is heads,
//! even is tails" works because 6 divides evenly by 2. Now suppose you want a
//! fair *four*-sided result from that die: "take the roll modulo 4" gives
//!
//! | roll | 1 | 2 | 3 | 4 | 5 | 6 |
//! |---|---|---|---|---|---|---|
//! | `roll mod 4` | 1 | 2 | 3 | 0 | 1 | 2 |
//!
//! and now `1` and `2` come up twice as often as `0` and `3`. The bias appears
//! because 4 does not divide 6 — the last two faces are leftovers.
//!
//! The fix is the obvious one: **throw the leftovers away**. Roll again if you
//! get a 5 or a 6. That is exactly what this crate does, with a 64-bit word in
//! place of a die:
//!
//! ```text
//!   0                                    bound                   2^64
//!   |------------- a whole number of n-sized blocks -------|-- leftovers --|
//!         word here  ->  accept, use  word mod n                 word here  ->  discard
//! ```
//!
//! where `bound = floor(2^64 / n) * n`. Below `bound` every residue appears
//! the same number of times, so `word mod n` is *exactly* uniform — not
//! approximately, not negligibly-biased. See `acceptance_bound` in the source.
//!
//! Discarding means the sampler can ask for more randomness than one word per
//! character, so the caller must reserve extra. With `n <= 89` the leftover
//! region is at most 88 values out of 2^64, so the chance of discarding any
//! given word is below `4.8e-18` — you would expect to wait longer than the
//! age of the universe to see one. [`SPARE_ENTROPY_WORDS`] covers eight of
//! them anyway, and running out is an error rather than a fallback to the
//! biased path. A fallback would mean the one branch nobody ever exercises is
//! the one that silently weakens the output.
//!
//! ## Idea three: the alphabet is a table, and every character has one home
//!
//! Four classes, selected independently. `--exclude-ambiguous` removes six
//! characters that get misread off a screen, and each of the six belongs to
//! exactly one class, so the flag can never empty a class that was selected:
//!
//! | class | full | ambiguous removed | remaining |
//! |---|---:|---|---:|
//! | lowercase | 26 | `l` | 25 |
//! | uppercase | 26 | `I`, `O` | 24 |
//! | digits | 10 | `0`, `1` | 8 |
//! | symbols | 27 | `\|` | 26 |
//!
//! # Usage
//!
//! ```
//! use coding_adventures_vault_pm_password_policy::{
//!     generate_password, CharacterClassesV1, PasswordPolicyV1,
//! };
//!
//! let policy = PasswordPolicyV1::new(24, CharacterClassesV1::all(), false)
//!     .expect("24 characters over all four classes is 155 bits");
//!
//! // The caller supplies the randomness. This crate never sources any.
//! let reserve = vec![0x5a; policy.required_entropy_bytes()];
//! let password = generate_password(&policy, &reserve).expect("exact reserve");
//! assert_eq!(password.len(), 24);
//! ```

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use coding_adventures_zeroize::Zeroizing;
use core::fmt::{self, Display, Formatter};

/// The lowercase Latin letters, in order.
pub const LOWERCASE_ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz";

/// The uppercase Latin letters, in order.
pub const UPPERCASE_ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";

/// The decimal digits, in order.
pub const DIGIT_ALPHABET: &[u8] = b"0123456789";

/// The accepted punctuation, in ASCII order.
///
/// These are the 32 printable US-ASCII punctuation characters minus `"`, `'`,
/// `` ` ``, `\`, and `/`. The exclusions are not decoration. Those five are the
/// characters most likely to survive a trip through a shell, a CSV import, a
/// JSON blob, or a hand-written SQL string as *something other than
/// themselves*, and a generated password that a downstream system silently
/// mangles is a password its owner can no longer log in with. The space
/// character is printable but is not punctuation, and was never a candidate:
/// it is invisible at both ends of a value and is stripped by a large fraction
/// of login forms.
pub const SYMBOL_ALPHABET: &[u8] = b"!#$%&()*+,-.:;<=>?@[]^_{|}~";

/// The characters `--exclude-ambiguous` removes.
///
/// `0`/`O`, `1`/`l`/`I`, and `|` against the same trio. Each belongs to exactly
/// one class, so removing all six leaves every class non-empty.
pub const AMBIGUOUS_CHARACTERS: &[u8] = b"01IOl|";

/// The minimum entropy, in bits, this product will mint a password at.
///
/// The number is argued in `VLT-PM44-cli-password-generate.md` §4.3 against
/// offline attack on a leaked credential database rather than against online
/// guessing: `2^80` guesses is centuries at `10^14` attempts per second, while
/// 64 bits is under a day. It is a floor and not a recommendation — the
/// default policy is nearly twice it.
pub const MIN_PASSWORD_ENTROPY_BITS: u32 = 80;

/// The length used when no `--length` is given: 155 bits over all four classes.
pub const DEFAULT_PASSWORD_LENGTH: usize = 24;

/// The shortest structurally accepted length.
///
/// A length of one is still refused, by the entropy floor rather than by this
/// bound. The two checks answer different questions and are kept separate on
/// purpose: this one asks whether the request is *shaped* like a password,
/// [`meets_minimum_entropy`] asks whether it is *worth* minting.
pub const MIN_PASSWORD_LENGTH: usize = 1;

/// The longest accepted length.
///
/// This exists to bound the single entropy reservation the caller makes — at
/// most `(128 + 8) * 8 = 1088` bytes — not because 128 characters means
/// anything.
pub const MAX_PASSWORD_LENGTH: usize = 128;

/// Bytes in one randomness word consumed by the sampler.
pub const ENTROPY_WORD_BYTES: usize = 8;

/// Extra randomness words reserved to cover discards.
///
/// See the module documentation: a word is discarded with probability below
/// `4.8e-18`, so eight spares is an enormous margin over an event that will
/// never be observed. They are reserved anyway because "never observed" is not
/// "cannot happen", and the alternative to having spares is falling back to a
/// biased draw.
pub const SPARE_ENTROPY_WORDS: usize = 8;

/// Closed, payload-free password-policy failures.
///
/// Every variant names a rule that was broken. None carries the requested
/// length, the alphabet, a bit count, or any part of a generated value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PasswordPolicyError {
    /// Every character class was disabled, leaving nothing to draw from.
    NoCharacterClass,
    /// The requested length was outside `MIN_PASSWORD_LENGTH..=MAX_PASSWORD_LENGTH`.
    LengthOutOfRange,
    /// The policy would produce fewer than [`MIN_PASSWORD_ENTROPY_BITS`] bits.
    EntropyFloorNotMet,
    /// The supplied randomness was not exactly [`PasswordPolicyV1::required_entropy_bytes`].
    EntropyBlockSizeMismatch,
    /// Every reserved word was discarded before the password was complete.
    EntropyExhausted,
}

impl Display for PasswordPolicyError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NoCharacterClass => "vault-pm password policy: no character class selected",
            Self::LengthOutOfRange => "vault-pm password policy: length out of range",
            Self::EntropyFloorNotMet => "vault-pm password policy: below the minimum entropy floor",
            Self::EntropyBlockSizeMismatch => "vault-pm password policy: wrong randomness size",
            Self::EntropyExhausted => "vault-pm password policy: randomness reserve exhausted",
        })
    }
}

impl std::error::Error for PasswordPolicyError {}

/// Which character classes a policy draws from.
///
/// All four are selected by default; the CLI's `--no-*` flags clear them one at
/// a time. Clearing all four is an error rather than an empty alphabet, because
/// an empty alphabet has no honest interpretation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CharacterClassesV1 {
    /// Draw from `a`–`z`.
    pub lowercase: bool,
    /// Draw from `A`–`Z`.
    pub uppercase: bool,
    /// Draw from `0`–`9`.
    pub digits: bool,
    /// Draw from [`SYMBOL_ALPHABET`].
    pub symbols: bool,
}

impl CharacterClassesV1 {
    /// Select every class — the default policy.
    #[must_use]
    pub const fn all() -> Self {
        Self {
            lowercase: true,
            uppercase: true,
            digits: true,
            symbols: true,
        }
    }

    /// Whether at least one class is selected.
    #[must_use]
    pub const fn any(self) -> bool {
        self.lowercase || self.uppercase || self.digits || self.symbols
    }
}

impl Default for CharacterClassesV1 {
    fn default() -> Self {
        Self::all()
    }
}

/// One validated password policy: a length and the exact alphabet to draw from.
///
/// A value of this type is proof that the policy passed every check in
/// `VLT-PM44` §3 and §4.3. There is no way to construct one that is under the
/// entropy floor, so [`generate_password`] never has to re-check it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PasswordPolicyV1 {
    length: usize,
    alphabet: Vec<u8>,
}

impl PasswordPolicyV1 {
    /// Validate one policy, or say exactly which rule it broke.
    ///
    /// The checks run in a fixed order — shape, then alphabet, then strength —
    /// so a request that is wrong in more than one way reports the most basic
    /// problem first.
    ///
    /// # Errors
    ///
    /// - [`PasswordPolicyError::LengthOutOfRange`] outside 1–128.
    /// - [`PasswordPolicyError::NoCharacterClass`] when every class is cleared.
    /// - [`PasswordPolicyError::EntropyFloorNotMet`] when
    ///   `alphabet^length < 2^80`.
    pub fn new(
        length: usize,
        classes: CharacterClassesV1,
        exclude_ambiguous: bool,
    ) -> Result<Self, PasswordPolicyError> {
        if !(MIN_PASSWORD_LENGTH..=MAX_PASSWORD_LENGTH).contains(&length) {
            return Err(PasswordPolicyError::LengthOutOfRange);
        }
        if !classes.any() {
            return Err(PasswordPolicyError::NoCharacterClass);
        }
        let alphabet = assemble_alphabet(classes, exclude_ambiguous);
        if !meets_minimum_entropy(alphabet.len(), length) {
            return Err(PasswordPolicyError::EntropyFloorNotMet);
        }
        Ok(Self { length, alphabet })
    }

    /// How many characters this policy produces.
    #[must_use]
    pub const fn length(&self) -> usize {
        self.length
    }

    /// The exact ordered alphabet drawn from.
    ///
    /// Not secret: it is a function of the flags the person typed, and it is
    /// exposed so tests can assert composition directly rather than inferring
    /// it from generated output.
    #[must_use]
    pub fn alphabet(&self) -> &[u8] {
        &self.alphabet
    }

    /// The exact number of random bytes [`generate_password`] requires.
    ///
    /// One 8-byte word per character, plus [`SPARE_ENTROPY_WORDS`] to cover
    /// discards. The caller reserves this in a single request so that a
    /// password is never assembled from randomness collected at two different
    /// moments.
    #[must_use]
    pub const fn required_entropy_bytes(&self) -> usize {
        (self.length + SPARE_ENTROPY_WORDS) * ENTROPY_WORD_BYTES
    }
}

/// Whether `alphabet_len^length` reaches `2^MIN_PASSWORD_ENTROPY_BITS`.
///
/// The integer form of "is this policy worth at least 80 bits?". It multiplies
/// and stops the instant the target is passed, so the accumulator never exceeds
/// `2^80 * 89 < 2^87` for any alphabet this crate can build, and
/// `saturating_mul` keeps it total even for a caller-invented alphabet size.
///
/// An alphabet of fewer than two characters carries no entropy at all — every
/// draw returns the same character no matter how long the password is — so it
/// is rejected outright instead of being multiplied a hundred times.
#[must_use]
pub fn meets_minimum_entropy(alphabet_len: usize, length: usize) -> bool {
    let Ok(alphabet_len) = u128::try_from(alphabet_len) else {
        return false;
    };
    if alphabet_len < 2 {
        return false;
    }
    let target: u128 = 1u128 << MIN_PASSWORD_ENTROPY_BITS;
    let mut reachable_strings: u128 = 1;
    for _ in 0..length {
        reachable_strings = reachable_strings.saturating_mul(alphabet_len);
        if reachable_strings >= target {
            return true;
        }
    }
    false
}

/// Turn exactly `policy.required_entropy_bytes()` random bytes into a password.
///
/// Deterministic: the same policy and the same bytes always produce the same
/// string. That is what makes the sampler testable, and it is safe precisely
/// because this crate cannot produce the bytes — they come from the operating
/// system, once, and are wiped afterwards.
///
/// The returned string is allocated at its exact final capacity before the
/// first character is pushed. That is not a micro-optimization: a `String` that
/// grows copies its contents to a fresh allocation and leaves the old buffer
/// behind unwiped, so a reallocation would strand a plaintext prefix of the
/// password on the heap where nothing will ever clear it.
///
/// # Errors
///
/// - [`PasswordPolicyError::EntropyBlockSizeMismatch`] if the buffer is not
///   exactly the required size. Both a short and an over-long buffer are
///   refused: a short one cannot be served, and an over-long one means the
///   caller and this function disagree about the reserve, which is not a
///   disagreement to paper over.
/// - [`PasswordPolicyError::EntropyExhausted`] if every reserved word,
///   including the spares, fell in the discard region.
pub fn generate_password(
    policy: &PasswordPolicyV1,
    entropy: &[u8],
) -> Result<Zeroizing<String>, PasswordPolicyError> {
    if entropy.len() != policy.required_entropy_bytes() {
        return Err(PasswordPolicyError::EntropyBlockSizeMismatch);
    }
    let alphabet = policy.alphabet();
    let modulus =
        u64::try_from(alphabet.len()).expect("an assembled alphabet is at most 89 characters wide");
    let bound = acceptance_bound(modulus);
    let mut words = entropy.chunks_exact(ENTROPY_WORD_BYTES);
    let mut password = Zeroizing::new(String::with_capacity(policy.length()));
    for _ in 0..policy.length() {
        let residue = loop {
            let Some(word) = words.next() else {
                return Err(PasswordPolicyError::EntropyExhausted);
            };
            let mut buffer = [0_u8; ENTROPY_WORD_BYTES];
            buffer.copy_from_slice(word);
            let draw = u64::from_be_bytes(buffer);
            if u128::from(draw) < bound {
                break draw % modulus;
            }
        };
        let index =
            usize::try_from(residue).expect("a residue modulo the alphabet length fits in usize");
        password.push(char::from(alphabet[index]));
    }
    Ok(password)
}

/// The largest multiple of `modulus` that fits in a 64-bit word.
///
/// Words strictly below this value cover every residue the same number of
/// times, which is what makes `draw % modulus` exactly uniform. Words at or
/// above it are the leftovers from the module documentation's die, and are
/// discarded.
///
/// The arithmetic is done in `u128` because the span being divided is `2^64`,
/// which a `u64` cannot hold — computing the bound in 64-bit arithmetic is the
/// classic way to get this one wrong by exactly one block.
fn acceptance_bound(modulus: u64) -> u128 {
    let span: u128 = 1u128 << 64;
    let modulus = u128::from(modulus);
    (span / modulus) * modulus
}

/// Concatenate the selected classes, minus the ambiguous characters.
///
/// Order is fixed — lowercase, uppercase, digits, symbols — so an alphabet is a
/// pure function of the flags and can be asserted character for character in a
/// test. Order has no effect on the output distribution, because every index is
/// equally likely.
fn assemble_alphabet(classes: CharacterClassesV1, exclude_ambiguous: bool) -> Vec<u8> {
    let mut alphabet = Vec::with_capacity(
        LOWERCASE_ALPHABET.len()
            + UPPERCASE_ALPHABET.len()
            + DIGIT_ALPHABET.len()
            + SYMBOL_ALPHABET.len(),
    );
    let selected = [
        (classes.lowercase, LOWERCASE_ALPHABET),
        (classes.uppercase, UPPERCASE_ALPHABET),
        (classes.digits, DIGIT_ALPHABET),
        (classes.symbols, SYMBOL_ALPHABET),
    ];
    for (enabled, class) in selected {
        if !enabled {
            continue;
        }
        alphabet.extend(
            class
                .iter()
                .copied()
                .filter(|byte| !exclude_ambiguous || !AMBIGUOUS_CHARACTERS.contains(byte)),
        );
    }
    alphabet
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// A deterministic test-only stream, used where a *distribution* is under
    /// test rather than an exact string.
    ///
    /// This is SplitMix64. It is emphatically not a cryptographic generator and
    /// is never compiled into the library — it exists so that the statistical
    /// sanity check in this module is reproducible on every run and on every
    /// machine, which a real CSPRNG could not be. The production entropy path
    /// is the operating-system CSPRNG and nothing else.
    struct SplitMix64(u64);

    impl SplitMix64 {
        fn next_u64(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            z ^ (z >> 31)
        }

        fn bytes(&mut self, count: usize) -> Vec<u8> {
            let mut out = Vec::with_capacity(count);
            while out.len() < count {
                out.extend_from_slice(&self.next_u64().to_be_bytes());
            }
            out.truncate(count);
            out
        }
    }

    fn classes(
        lowercase: bool,
        uppercase: bool,
        digits: bool,
        symbols: bool,
    ) -> CharacterClassesV1 {
        CharacterClassesV1 {
            lowercase,
            uppercase,
            digits,
            symbols,
        }
    }

    /// Unwrap the refusal a generation was supposed to produce.
    ///
    /// Tests cannot write `assert_eq!(generate_password(..), Err(..))`, because
    /// the success arm is a `Zeroizing<String>` and that type implements
    /// neither `Debug` nor `PartialEq` — a secret must not be printable by an
    /// assertion failure or comparable in non-constant time. That restriction
    /// is a property worth keeping, so the tests bend around it rather than
    /// asking for derives on the wrapper.
    fn refusal(result: Result<Zeroizing<String>, PasswordPolicyError>) -> PasswordPolicyError {
        match result {
            Ok(_) => panic!("expected a refusal, but a password was generated"),
            Err(error) => error,
        }
    }

    #[test]
    fn class_tables_have_the_documented_sizes_and_no_overlap() {
        assert_eq!(LOWERCASE_ALPHABET.len(), 26);
        assert_eq!(UPPERCASE_ALPHABET.len(), 26);
        assert_eq!(DIGIT_ALPHABET.len(), 10);
        assert_eq!(SYMBOL_ALPHABET.len(), 27);
        assert_eq!(AMBIGUOUS_CHARACTERS.len(), 6);

        let mut seen = BTreeSet::new();
        for class in [
            LOWERCASE_ALPHABET,
            UPPERCASE_ALPHABET,
            DIGIT_ALPHABET,
            SYMBOL_ALPHABET,
        ] {
            for byte in class {
                assert!(byte.is_ascii_graphic(), "{byte} is not printable ASCII");
                assert!(seen.insert(*byte), "{byte} appears in two classes");
            }
        }
        assert_eq!(seen.len(), 89);

        // The five characters most likely to be mangled downstream, plus space.
        for excluded in b"\"'`\\/ " {
            assert!(!seen.contains(excluded), "{excluded} should be excluded");
        }
    }

    #[test]
    fn every_ambiguous_character_belongs_to_exactly_one_class() {
        for ambiguous in AMBIGUOUS_CHARACTERS {
            let homes = [
                LOWERCASE_ALPHABET,
                UPPERCASE_ALPHABET,
                DIGIT_ALPHABET,
                SYMBOL_ALPHABET,
            ]
            .into_iter()
            .filter(|class| class.contains(ambiguous))
            .count();
            assert_eq!(homes, 1, "{ambiguous} has {homes} homes");
        }
    }

    #[test]
    fn alphabet_composition_matches_the_specification_table() {
        let table: [(CharacterClassesV1, bool, usize); 12] = [
            (classes(true, true, true, true), false, 89),
            (classes(true, true, true, true), true, 83),
            (classes(true, true, true, false), false, 62),
            (classes(true, true, true, false), true, 57),
            (classes(true, true, false, false), false, 52),
            (classes(true, true, false, false), true, 49),
            (classes(true, false, true, false), false, 36),
            (classes(true, false, true, false), true, 33),
            (classes(true, false, false, false), false, 26),
            (classes(false, false, true, false), false, 10),
            (classes(false, false, true, false), true, 8),
            (classes(false, false, false, true), false, 27),
        ];
        for (selected, exclude_ambiguous, expected) in table {
            let alphabet = assemble_alphabet(selected, exclude_ambiguous);
            assert_eq!(
                alphabet.len(),
                expected,
                "{selected:?} exclude_ambiguous={exclude_ambiguous}"
            );
            let unique: BTreeSet<u8> = alphabet.iter().copied().collect();
            assert_eq!(unique.len(), alphabet.len(), "duplicate characters");
            if exclude_ambiguous {
                for ambiguous in AMBIGUOUS_CHARACTERS {
                    assert!(!alphabet.contains(ambiguous), "{ambiguous} survived");
                }
            }
        }
    }

    #[test]
    fn excluding_ambiguous_characters_never_empties_a_selected_class() {
        let per_class = [
            (classes(true, false, false, false), 25),
            (classes(false, true, false, false), 24),
            (classes(false, false, true, false), 8),
            (classes(false, false, false, true), 26),
        ];
        for (selected, expected) in per_class {
            assert_eq!(assemble_alphabet(selected, true).len(), expected);
        }
    }

    #[test]
    fn the_entropy_floor_is_exact_on_both_sides_of_every_documented_row() {
        // (alphabet size, first accepted length). One below must be refused.
        let table = [
            (89_usize, 13_usize),
            (83, 13),
            (62, 14),
            (52, 15),
            (36, 16),
            (26, 18),
            (10, 25),
            (8, 27),
        ];
        for (alphabet_len, minimum) in table {
            assert!(
                meets_minimum_entropy(alphabet_len, minimum),
                "{alphabet_len}^{minimum} should reach the floor"
            );
            assert!(
                !meets_minimum_entropy(alphabet_len, minimum - 1),
                "{alphabet_len}^{} should miss the floor",
                minimum - 1
            );
        }
    }

    #[test]
    fn a_degenerate_alphabet_carries_no_entropy_at_any_length() {
        assert!(!meets_minimum_entropy(0, MAX_PASSWORD_LENGTH));
        assert!(!meets_minimum_entropy(1, MAX_PASSWORD_LENGTH));
        assert!(!meets_minimum_entropy(1, usize::MAX));
        assert!(meets_minimum_entropy(2, 80));
        assert!(!meets_minimum_entropy(2, 79));
    }

    #[test]
    fn policy_validation_reports_the_most_basic_problem_first() {
        assert_eq!(
            PasswordPolicyV1::new(0, CharacterClassesV1::all(), false),
            Err(PasswordPolicyError::LengthOutOfRange)
        );
        assert_eq!(
            PasswordPolicyV1::new(MAX_PASSWORD_LENGTH + 1, CharacterClassesV1::all(), false),
            Err(PasswordPolicyError::LengthOutOfRange)
        );
        // Length is checked before classes: a request that is wrong twice
        // reports the shape problem, not the alphabet one.
        assert_eq!(
            PasswordPolicyV1::new(0, classes(false, false, false, false), false),
            Err(PasswordPolicyError::LengthOutOfRange)
        );
        assert_eq!(
            PasswordPolicyV1::new(24, classes(false, false, false, false), false),
            Err(PasswordPolicyError::NoCharacterClass)
        );
        assert_eq!(
            PasswordPolicyV1::new(12, CharacterClassesV1::all(), false),
            Err(PasswordPolicyError::EntropyFloorNotMet),
            "12 characters over 89 is 77.7 bits and must be refused"
        );
        assert!(PasswordPolicyV1::new(13, CharacterClassesV1::all(), false).is_ok());
    }

    #[test]
    fn the_default_policy_is_accepted_and_reserves_the_documented_size() {
        let policy = PasswordPolicyV1::new(
            DEFAULT_PASSWORD_LENGTH,
            CharacterClassesV1::default(),
            false,
        )
        .expect("the default policy must be valid");
        assert_eq!(policy.length(), 24);
        assert_eq!(policy.alphabet().len(), 89);
        assert_eq!(policy.required_entropy_bytes(), (24 + 8) * 8);

        let longest = PasswordPolicyV1::new(MAX_PASSWORD_LENGTH, CharacterClassesV1::all(), false)
            .expect("128 characters is valid");
        assert_eq!(longest.required_entropy_bytes(), 1088);
    }

    #[test]
    fn generation_is_deterministic_and_stays_inside_the_alphabet() {
        let policy = PasswordPolicyV1::new(24, CharacterClassesV1::all(), false).unwrap();
        let reserve = SplitMix64(0x0123_4567_89ab_cdef).bytes(policy.required_entropy_bytes());

        let first = generate_password(&policy, &reserve).expect("exact reserve");
        let second = generate_password(&policy, &reserve).expect("exact reserve");
        assert_eq!(
            first.as_str(),
            second.as_str(),
            "sampling must be a function"
        );
        assert_eq!(first.len(), 24);
        for byte in first.as_bytes() {
            assert!(policy.alphabet().contains(byte), "{byte} left the alphabet");
        }
    }

    #[test]
    fn excluded_characters_never_appear_in_output() {
        let policy = PasswordPolicyV1::new(128, CharacterClassesV1::all(), true).unwrap();
        let mut stream = SplitMix64(0xfeed_face_dead_beef);
        for _ in 0..64 {
            let reserve = stream.bytes(policy.required_entropy_bytes());
            let password = generate_password(&policy, &reserve).unwrap();
            for ambiguous in AMBIGUOUS_CHARACTERS {
                assert!(
                    !password.as_bytes().contains(ambiguous),
                    "{ambiguous} appeared under --exclude-ambiguous"
                );
            }
        }
    }

    #[test]
    fn a_narrowed_policy_draws_only_from_its_classes() {
        let policy = PasswordPolicyV1::new(64, classes(false, false, true, false), false).unwrap();
        let reserve = SplitMix64(7).bytes(policy.required_entropy_bytes());
        let password = generate_password(&policy, &reserve).unwrap();
        assert!(password.chars().all(|character| character.is_ascii_digit()));
    }

    #[test]
    fn the_reserve_size_must_match_exactly_in_both_directions() {
        let policy = PasswordPolicyV1::new(24, CharacterClassesV1::all(), false).unwrap();
        let exact = policy.required_entropy_bytes();
        for wrong in [0, 1, exact - 1, exact + 1] {
            let buffer = vec![0x11; wrong];
            assert_eq!(
                refusal(generate_password(&policy, &buffer)),
                PasswordPolicyError::EntropyBlockSizeMismatch,
                "a {wrong}-byte reserve must be refused"
            );
        }
        let buffer = vec![0x11; exact];
        assert!(generate_password(&policy, &buffer).is_ok());
    }

    #[test]
    fn acceptance_bound_is_the_largest_multiple_of_the_modulus() {
        for modulus in [2_u64, 8, 10, 26, 27, 36, 52, 62, 83, 89] {
            let bound = acceptance_bound(modulus);
            assert_eq!(bound % u128::from(modulus), 0, "bound must be a multiple");
            assert!(bound <= 1u128 << 64);
            assert!(
                bound + u128::from(modulus) > 1u128 << 64,
                "bound must be the largest such multiple"
            );
        }
        // A modulus that divides 2^64 evenly wastes nothing at all.
        assert_eq!(acceptance_bound(8), 1u128 << 64);
        assert_eq!(acceptance_bound(2), 1u128 << 64);
    }

    #[test]
    fn words_in_the_discard_region_are_skipped_rather_than_reduced() {
        // Alphabet of 89. The discard region is [bound, 2^64), which is the
        // top `2^64 mod 89` words. Feed one of those first and prove the
        // character comes from the *second* word instead.
        let policy = PasswordPolicyV1::new(13, CharacterClassesV1::all(), false).unwrap();
        let modulus = u64::try_from(policy.alphabet().len()).unwrap();
        let bound = acceptance_bound(modulus);
        let discarded = u64::MAX;
        assert!(
            u128::from(discarded) >= bound,
            "u64::MAX must fall in the discard region for 89"
        );

        let mut with_discard = Vec::new();
        with_discard.extend_from_slice(&discarded.to_be_bytes());
        let mut baseline = Vec::new();
        // 13 usable words, all distinct so the comparison is not accidental.
        for index in 0..13_u64 {
            let word = index.wrapping_mul(0x0100_0000_0000_0001);
            assert!(u128::from(word) < bound);
            with_discard.extend_from_slice(&word.to_be_bytes());
            baseline.extend_from_slice(&word.to_be_bytes());
        }
        let padding = policy.required_entropy_bytes();
        with_discard.resize(padding, 0);
        baseline.resize(padding, 0);

        let skipped = generate_password(&policy, &with_discard).unwrap();
        let plain = generate_password(&policy, &baseline).unwrap();
        assert_eq!(
            skipped.as_str(),
            plain.as_str(),
            "a discarded word must contribute nothing at all"
        );
        // And a biased implementation *would* have used it, visibly: reducing
        // the discarded word instead of skipping it selects an ordinary index,
        // and a different one than the word that actually got used. So this
        // test would fail against a `word % n` sampler rather than passing by
        // luck.
        let biased_index = usize::try_from(discarded % modulus).unwrap();
        let honest_index = usize::try_from(0_u64 % modulus).unwrap();
        assert_ne!(
            policy.alphabet()[biased_index],
            policy.alphabet()[honest_index],
            "the discarded word must not select the same character as the used one"
        );
        assert_eq!(plain.as_bytes()[0], policy.alphabet()[honest_index]);
    }

    #[test]
    fn an_exhausted_reserve_fails_instead_of_falling_back() {
        let policy = PasswordPolicyV1::new(13, CharacterClassesV1::all(), false).unwrap();
        // Every single word lands in the discard region, spares included.
        let reserve = vec![0xff; policy.required_entropy_bytes()];
        assert_eq!(
            refusal(generate_password(&policy, &reserve)),
            PasswordPolicyError::EntropyExhausted
        );
    }

    #[test]
    fn the_spare_words_absorb_discards_up_to_their_count() {
        let policy = PasswordPolicyV1::new(13, CharacterClassesV1::all(), false).unwrap();
        let mut reserve = Vec::new();
        for _ in 0..SPARE_ENTROPY_WORDS {
            reserve.extend_from_slice(&u64::MAX.to_be_bytes());
        }
        reserve.resize(policy.required_entropy_bytes(), 0);
        let password = generate_password(&policy, &reserve).expect("eight discards are survivable");
        assert_eq!(password.len(), 13);

        // One more discard than there are spares is one too many.
        let mut over = Vec::new();
        for _ in 0..=SPARE_ENTROPY_WORDS {
            over.extend_from_slice(&u64::MAX.to_be_bytes());
        }
        over.resize(policy.required_entropy_bytes(), 0);
        assert_eq!(
            refusal(generate_password(&policy, &over)),
            PasswordPolicyError::EntropyExhausted
        );
    }

    #[test]
    fn output_is_not_obviously_non_random() {
        let policy = PasswordPolicyV1::new(64, CharacterClassesV1::all(), false).unwrap();
        let alphabet_len = policy.alphabet().len();
        let samples = 400_usize;
        let mut stream = SplitMix64(0xdead_beef_cafe_0042);
        let mut counts = [0_usize; 128];
        let mut distinct = BTreeSet::new();

        for _ in 0..samples {
            let reserve = stream.bytes(policy.required_entropy_bytes());
            let password = generate_password(&policy, &reserve).unwrap();
            assert_eq!(password.len(), 64);
            for byte in password.as_bytes() {
                counts[usize::from(*byte)] += 1;
            }
            // Consecutive outputs must differ, and no single output may be a
            // short repeating cycle — the two shapes a broken generator that
            // "looks random" most often takes.
            for period in 1..=8 {
                let repeats = password
                    .as_bytes()
                    .windows(period + 1)
                    .all(|window| window[0] == window[period]);
                assert!(!repeats, "output cycles with period {period}");
            }
            assert!(distinct.insert(password.as_str().to_owned()), "repeat draw");
        }

        let drawn = samples * 64;
        let expected = drawn / alphabet_len;
        for member in policy.alphabet() {
            let observed = counts[usize::from(*member)];
            assert!(observed > 0, "{member} never appeared in {drawn} draws");
            assert!(
                observed * 2 > expected && observed < expected * 2,
                "{member} appeared {observed} times against an expectation of {expected}"
            );
        }
        // Nothing outside the alphabet was ever emitted.
        let emitted: usize = counts.iter().sum();
        let inside: usize = policy
            .alphabet()
            .iter()
            .map(|member| counts[usize::from(*member)])
            .sum();
        assert_eq!(emitted, inside);
    }

    #[test]
    fn errors_are_payload_free_and_displayable() {
        let messages = [
            PasswordPolicyError::NoCharacterClass,
            PasswordPolicyError::LengthOutOfRange,
            PasswordPolicyError::EntropyFloorNotMet,
            PasswordPolicyError::EntropyBlockSizeMismatch,
            PasswordPolicyError::EntropyExhausted,
        ];
        let mut seen = BTreeSet::new();
        for error in messages {
            let rendered = error.to_string();
            assert!(rendered.starts_with("vault-pm password policy: "));
            assert!(rendered.is_ascii());
            assert!(seen.insert(rendered), "two errors share one message");
        }
        assert_eq!(
            format!("{:?}", PasswordPolicyError::EntropyExhausted),
            "EntropyExhausted"
        );
    }

    #[test]
    fn class_selection_helpers_behave() {
        assert!(CharacterClassesV1::all().any());
        assert!(!classes(false, false, false, false).any());
        assert!(classes(false, false, false, true).any());
        assert_eq!(CharacterClassesV1::default(), CharacterClassesV1::all());
    }
}
