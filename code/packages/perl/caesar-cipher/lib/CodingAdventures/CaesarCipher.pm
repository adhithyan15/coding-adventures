package CodingAdventures::CaesarCipher;

# ============================================================================
# Caesar Cipher -- the oldest substitution cipher
#
# Julius Caesar used this cipher around 58 BCE to protect military
# correspondence during the Gallic Wars. The idea is beautifully simple:
# each letter in the plaintext is replaced by a letter a fixed number of
# positions down the alphabet.
#
# For example, with a shift of 3:
#
#   Plaintext:   A B C D E F G H I J K L M N O P Q R S T U V W X Y Z
#   Ciphertext:  D E F G H I J K L M N O P Q R S T U V W X Y Z A B C
#
# So "HELLO" becomes "KHOOR", and "ATTACK AT DAWN" becomes "DWWDFN DW GDZQ".
#
# The cipher wraps around: after Z comes A again. Mathematically, for a
# shift `s`, each letter at position `p` (A=0, B=1, ..., Z=25) maps to:
#
#   encrypted_position = (p + s) mod 26
#   decrypted_position = (p - s) mod 26
#
# Non-alphabetic characters (digits, punctuation, spaces) pass through
# unchanged -- only the 26 English letters are shifted.
#
# ============================================================================
# Why is it so easy to break?
#
# The Caesar cipher has only 25 possible keys (shifts 1-25; shift 0 is
# the identity). An attacker can simply try all 25 and look for readable
# text -- this is called "brute force".
#
# A more sophisticated approach uses "frequency analysis": in English,
# the letter E appears about 12.7% of the time, T about 9.1%, and so on.
# By comparing the frequency distribution of letters in the ciphertext
# to known English frequencies, we can guess the shift without trying
# all possibilities.
#
# ============================================================================
# This module is part of the coding-adventures project, an educational
# computing stack built from logic gates up through interpreters and
# compilers.
# ============================================================================

use strict;
use warnings;
use v5.20;

our $VERSION = '0.1.0';

# ============================================================================
# English Letter Frequencies
# ============================================================================
#
# These frequencies come from large-scale analysis of English text. They
# represent the probability of each letter appearing in a typical English
# document. We use them later for frequency analysis to crack ciphertexts.
#
# The values are approximate -- different corpora give slightly different
# numbers -- but these are widely accepted reference values:
#
#   E is the most common letter (~12.7%)
#   T is second (~9.1%)
#   Z and Q are rarest (~0.1%)
#
# Fun fact: this distribution is why Wheel of Fortune gives you R, S, T,
# L, N, E for free in the bonus round -- they're among the most common!
# ============================================================================

my %ENGLISH_FREQUENCIES = (
    A => 0.082, B => 0.015, C => 0.028, D => 0.043,
    E => 0.127, F => 0.022, G => 0.020, H => 0.061,
    I => 0.070, J => 0.002, K => 0.008, L => 0.040,
    M => 0.024, N => 0.067, O => 0.075, P => 0.019,
    Q => 0.001, R => 0.060, S => 0.063, T => 0.091,
    U => 0.028, V => 0.010, W => 0.024, X => 0.002,
    Y => 0.020, Z => 0.001,
);


# ============================================================================
# encrypt($plaintext, $shift) -> $ciphertext
# ============================================================================
#
# Encrypts a string using the Caesar cipher with the given shift value.
#
# How it works, step by step:
#
#   1. We iterate over each character in the input string.
#   2. If the character is a letter (A-Z or a-z), we shift it forward
#      by `$shift` positions in the alphabet.
#   3. We use modular arithmetic (% 26) to wrap around: shifting Z by 1
#      gives A, not some character past Z.
#   4. We preserve the original case: uppercase stays uppercase, lowercase
#      stays lowercase.
#   5. Non-letter characters (spaces, digits, punctuation) pass through
#      unchanged.
#
# The shift is normalized to the range 0-25 using modulo. This means:
#   - A shift of 26 is the same as a shift of 0 (no change)
#   - A shift of 29 is the same as a shift of 3
#   - A shift of -1 is the same as a shift of 25
#
# Example:
#   encrypt("Hello, World!", 3)  => "Khoor, Zruog!"
#
# Under the hood, we use Perl's `ord()` to get a character's ASCII code,
# do the math, then use `chr()` to convert back. The ASCII codes are:
#   A=65, B=66, ..., Z=90
#   a=97, b=98, ..., z=122
#
# So for uppercase: position = ord($char) - 65 (gives 0-25)
#    shifted = (position + shift) % 26
#    result  = chr(shifted + 65)
# ============================================================================

sub encrypt {
    my ($plaintext, $shift) = @_;

    # Normalize the shift to 0-25 range. Perl's % operator can return
    # negative values for negative operands, so we add 26 first to
    # ensure a positive result.
    $shift = $shift % 26;

    # Process each character using split/map/join -- a classic Perl idiom.
    # split('', $str) breaks a string into individual characters,
    # map transforms each one, and join reassembles them.
    my $ciphertext = join '', map {
        if (/[A-Z]/) {
            # Uppercase letter: shift within the A-Z range (ASCII 65-90)
            chr( ( ord($_) - ord('A') + $shift ) % 26 + ord('A') )
        }
        elsif (/[a-z]/) {
            # Lowercase letter: shift within the a-z range (ASCII 97-122)
            chr( ( ord($_) - ord('a') + $shift ) % 26 + ord('a') )
        }
        else {
            # Non-alphabetic character: pass through unchanged
            # This preserves spaces, digits, punctuation, etc.
            $_
        }
    } split //, $plaintext;

    return $ciphertext;
}


# ============================================================================
# decrypt($ciphertext, $shift) -> $plaintext
# ============================================================================
#
# Decrypts a Caesar cipher by shifting in the opposite direction.
#
# Decryption is just encryption with the negated shift! If we encrypted
# with shift 3, we decrypt with shift -3 (or equivalently, shift 23,
# since -3 mod 26 = 23).
#
# This elegant symmetry comes from modular arithmetic:
#   encrypt: c = (p + s) mod 26
#   decrypt: p = (c - s) mod 26 = (c + (26 - s)) mod 26
#
# So decrypt(encrypt(text, s), s) always returns the original text.
#
# Example:
#   decrypt("Khoor, Zruog!", 3)  => "Hello, World!"
# ============================================================================

sub decrypt {
    my ($ciphertext, $shift) = @_;

    # Decryption is encryption with the negated shift.
    # We negate and let encrypt() handle the modular normalization.
    return encrypt($ciphertext, -$shift);
}


# ============================================================================
# rot13($text) -> $transformed
# ============================================================================
#
# ROT13 is a special case of the Caesar cipher with a shift of 13.
#
# What makes ROT13 unique is that it's its own inverse! Since the English
# alphabet has 26 letters and 13 is exactly half of 26:
#
#   rot13(rot13(text)) == text
#
# This means the same function both encrypts and decrypts. This property
# made ROT13 popular on Usenet forums in the 1980s and 1990s for hiding
# spoilers and punchlines -- you could read them by applying ROT13 again.
#
# The mapping looks like this:
#
#   A B C D E F G H I J K L M  <-->  N O P Q R S T U V W X Y Z
#
# So A<->N, B<->O, C<->P, and so on.
#
# Example:
#   rot13("Hello")   => "Uryyb"
#   rot13("Uryyb")   => "Hello"    # Self-inverse!
# ============================================================================

sub rot13 {
    my ($text) = @_;
    return encrypt($text, 13);
}


# ============================================================================
# brute_force($ciphertext) -> @results
# ============================================================================
#
# Tries all 25 possible shifts and returns the results.
#
# Since the Caesar cipher only has 25 meaningful keys (shifts 1 through
# 25; shift 0 just returns the original text), we can try every single
# one. This is the simplest form of cryptanalysis -- no cleverness needed,
# just raw enumeration.
#
# Returns a list of hash references, each containing:
#   - shift: the shift value used (1-25)
#   - text:  the decrypted text for that shift
#
# A human (or program) can then scan the results for readable text.
#
# Example:
#   my @results = brute_force("Khoor");
#   # $results[2] = { shift => 3, text => "Hello" }
#
# In practice, you'd combine this with a dictionary check or frequency
# analysis to automatically identify the correct shift. But for short
# texts, a human glance at 25 options is often fastest!
# ============================================================================

sub brute_force {
    my ($ciphertext) = @_;

    # Try every shift from 1 to 25 and collect the results.
    # We skip shift 0 because that's just the original ciphertext.
    my @results;
    for my $shift (1 .. 25) {
        push @results, {
            shift => $shift,
            text  => decrypt($ciphertext, $shift),
        };
    }

    return @results;
}


# ============================================================================
# frequency_analysis($ciphertext) -> $best_shift
# ============================================================================
#
# Uses letter frequency analysis to determine the most likely shift.
#
# The idea: in any sufficiently long English text, letters appear with
# predictable frequencies. E is most common (~12.7%), followed by T (~9.1%),
# A (~8.2%), and so on. Z and Q appear less than 0.1% of the time.
#
# When we encrypt with a Caesar cipher, we don't change the frequency
# distribution -- we just shift it. So if E (the most common letter in
# English) was shifted by 3, then H should be the most common letter
# in the ciphertext.
#
# Our algorithm:
#
#   1. Count how often each letter appears in the ciphertext.
#   2. For each possible shift (0-25), calculate how well the shifted
#      frequencies match known English frequencies.
#   3. We use "chi-squared" as our similarity metric -- lower is better.
#   4. Return the shift with the lowest chi-squared score.
#
# The chi-squared statistic:
#
#            (observed - expected)^2
#   chi^2 = SUM ---------------------
#                   expected
#
# This penalizes large deviations more than small ones, making it
# excellent for comparing frequency distributions.
#
# Limitations:
#   - Works best on longer texts (50+ characters of actual letters)
#   - Short texts may not have enough letters for reliable statistics
#   - Non-English text will give wrong results
#
# Example:
#   my $shift = frequency_analysis("Khoor Zruog");  # Returns 3
# ============================================================================

sub frequency_analysis {
    my ($ciphertext) = @_;

    # Step 1: Count letter frequencies in the ciphertext.
    # We convert everything to uppercase for counting since our
    # frequency table uses uppercase keys.
    my %counts;
    my $total = 0;

    for my $char (split //, uc $ciphertext) {
        if ($char =~ /[A-Z]/) {
            $counts{$char}++;
            $total++;
        }
    }

    # Edge case: if there are no letters at all, return 0 (no shift).
    return 0 if $total == 0;

    # Step 2: Try each possible shift and compute chi-squared score.
    my $best_shift = 0;
    my $best_score = ~0;  # Start with a very large number (bitwise NOT of 0)

    for my $shift (0 .. 25) {
        my $score = 0;

        # For each letter in the alphabet, compute how well it matches
        # if we assume this shift was used.
        for my $letter ('A' .. 'Z') {
            # If this shift is correct, then the letter at position
            # (letter + shift) mod 26 in the ciphertext should correspond
            # to the frequency of `letter` in English.
            my $shifted = chr( ( ord($letter) - ord('A') + $shift ) % 26 + ord('A') );
            my $observed = ($counts{$shifted} // 0) / $total;
            my $expected = $ENGLISH_FREQUENCIES{$letter};

            # Chi-squared: (observed - expected)^2 / expected
            # We guard against division by zero, though our frequency
            # table has no zero values.
            if ($expected > 0) {
                $score += ($observed - $expected) ** 2 / $expected;
            }
        }

        # Keep track of the shift with the lowest chi-squared score.
        if ($score < $best_score) {
            $best_score = $score;
            $best_shift = $shift;
        }
    }

    return $best_shift;
}


1;  # Perl modules must return a true value -- this is a quirk of Perl's
    # module loading system. `use` and `require` evaluate the file and
    # check that the last expression is true. If it's false, they die
    # with "... did not return a true value". The convention is to end
    # every .pm file with `1;`.

__END__

=head1 NAME

CodingAdventures::CaesarCipher - Caesar cipher with brute-force and frequency analysis

=head1 SYNOPSIS

    use CodingAdventures::CaesarCipher;

    # Encrypt and decrypt
    my $encrypted = CodingAdventures::CaesarCipher::encrypt("Hello, World!", 3);
    # "Khoor, Zruog!"

    my $decrypted = CodingAdventures::CaesarCipher::decrypt($encrypted, 3);
    # "Hello, World!"

    # ROT13 (self-inverse)
    my $hidden = CodingAdventures::CaesarCipher::rot13("spoiler");
    my $revealed = CodingAdventures::CaesarCipher::rot13($hidden);

    # Brute force all 25 shifts
    my @results = CodingAdventures::CaesarCipher::brute_force($encrypted);

    # Frequency analysis
    my $shift = CodingAdventures::CaesarCipher::frequency_analysis($encrypted);

=head1 DESCRIPTION

This module implements the Caesar cipher, one of the earliest known encryption
techniques. It provides encryption, decryption, ROT13, brute-force attack,
and frequency analysis for educational purposes.

=head1 VERSION

Version 0.1.0

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
