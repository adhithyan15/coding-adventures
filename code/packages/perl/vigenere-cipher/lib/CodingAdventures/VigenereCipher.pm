package CodingAdventures::VigenereCipher;

# ============================================================================
# CodingAdventures::VigenereCipher
# ============================================================================
#
# The Vigenere cipher is a *polyalphabetic substitution* cipher invented by
# Giovan Battista Bellaso in 1553 and later misattributed to Blaise de
# Vigenere. For 300 years it was considered "le chiffre indechiffrable"
# until Friedrich Kasiski published a general method for breaking it in 1863.
#
# How It Works (Encryption)
# -------------------------
#
# Unlike a Caesar cipher (one fixed shift), the Vigenere cipher uses a
# *keyword* to apply a different shift at each position:
#
#     Plaintext:  A  T  T  A  C  K  A  T  D  A  W  N
#     Keyword:    L  E  M  O  N  L  E  M  O  N  L  E
#     Shift:      11 4  12 14 13 11 4  12 14 13 11 4
#     Ciphertext: L  X  F  O  P  V  E  F  R  N  H  R
#
# Each plaintext letter is shifted forward by the amount indicated by the
# corresponding keyword letter (A=0, B=1, ... Z=25). Non-alphabetic
# characters pass through unchanged and do NOT advance the keyword position.
#
# How It Works (Decryption)
# -------------------------
#
# Reverse the process: shift each letter *backward* by the keyword amount.
#
# Cryptanalysis (Breaking the Cipher)
# ------------------------------------
#
# Step 1 -- Find the key length using the Index of Coincidence (IC).
# Step 2 -- Find each key letter using chi-squared analysis.

use strict;
use warnings;
use POSIX qw(ceil);
use Exporter 'import';

our $VERSION = '0.1.0';
our @EXPORT_OK = qw(encrypt decrypt find_key_length find_key break_cipher);

# ============================================================================
# English Letter Frequencies
# ============================================================================
#
# Expected frequencies for A-Z in typical English text. Used by the
# chi-squared test to determine the most likely shift for each key position.

my @ENGLISH_FREQ = (
    0.08167, 0.01492, 0.02782, 0.04253, 0.12702, 0.02228, # A-F
    0.02015, 0.06094, 0.06966, 0.00153, 0.00772, 0.04025, # G-L
    0.02406, 0.06749, 0.07507, 0.01929, 0.00095, 0.05987, # M-R
    0.06327, 0.09056, 0.02758, 0.00978, 0.02360, 0.00150, # S-X
    0.01974, 0.00074,                                       # Y-Z
);

# ============================================================================
# encrypt($plaintext, $key) -> $ciphertext
# ============================================================================
#
# Encrypt plaintext using the Vigenere cipher with the given key.
#
# Rules:
#   * Key must be non-empty and contain only A-Z / a-z.
#   * Uppercase letters stay uppercase; lowercase stay lowercase.
#   * Non-alphabetic characters pass through unchanged.
#   * The key position advances only on alphabetic characters.
#
# Example:
#   encrypt("ATTACKATDAWN", "LEMON") --> "LXFOPVEFRNHR"
#   encrypt("Hello, World!", "key")  --> "Rijvs, Uyvjn!"

sub encrypt {
    my ($plaintext, $key) = @_;

    die "Key must be a non-empty alphabetic string\n"
        unless defined $key && length($key) > 0 && $key =~ /^[A-Za-z]+$/;

    my @key_chars = split //, uc($key);
    my $key_len = scalar @key_chars;
    my $key_idx = 0;
    my $result = "";

    for my $ch (split //, $plaintext) {
        if ($ch =~ /[A-Za-z]/) {
            # Determine shift from current key letter (A=0, B=1, ..., Z=25)
            my $shift = ord($key_chars[$key_idx % $key_len]) - ord('A');

            # Apply shift, preserving case
            my $base = ($ch =~ /[A-Z]/) ? ord('A') : ord('a');
            $result .= chr((ord($ch) - $base + $shift) % 26 + $base);

            $key_idx++;
        } else {
            # Non-alpha passes through; key does NOT advance
            $result .= $ch;
        }
    }

    return $result;
}

# ============================================================================
# decrypt($ciphertext, $key) -> $plaintext
# ============================================================================
#
# Decrypt ciphertext by shifting each letter *backward* by the key amount.
# Exact inverse of encrypt.
#
# Example:
#   decrypt("LXFOPVEFRNHR", "LEMON") --> "ATTACKATDAWN"

sub decrypt {
    my ($ciphertext, $key) = @_;

    die "Key must be a non-empty alphabetic string\n"
        unless defined $key && length($key) > 0 && $key =~ /^[A-Za-z]+$/;

    my @key_chars = split //, uc($key);
    my $key_len = scalar @key_chars;
    my $key_idx = 0;
    my $result = "";

    for my $ch (split //, $ciphertext) {
        if ($ch =~ /[A-Za-z]/) {
            my $shift = ord($key_chars[$key_idx % $key_len]) - ord('A');

            # Shift backward (subtract), add 26 to avoid negative modulo
            my $base = ($ch =~ /[A-Z]/) ? ord('A') : ord('a');
            $result .= chr((ord($ch) - $base - $shift + 26) % 26 + $base);

            $key_idx++;
        } else {
            $result .= $ch;
        }
    }

    return $result;
}

# ============================================================================
# _index_of_coincidence($text) -> $ic
# ============================================================================
#
# The Index of Coincidence (IC) measures how likely it is that two randomly
# chosen letters from a text are the same. English IC ~ 0.0667; random ~ 0.0385.
#
# Formula: IC = sum(n_i * (n_i - 1)) / (N * (N - 1))

sub _index_of_coincidence {
    my ($text) = @_;
    my @counts = (0) x 26;
    my $total = 0;

    for my $ch (split //, $text) {
        if ($ch =~ /[A-Za-z]/) {
            my $idx = ord(uc($ch)) - ord('A');
            $counts[$idx]++;
            $total++;
        }
    }

    return 0 if $total <= 1;

    my $sum = 0;
    for my $c (@counts) {
        $sum += $c * ($c - 1);
    }

    return $sum / ($total * ($total - 1));
}

# ============================================================================
# find_key_length($ciphertext, $max_length?) -> $key_length
# ============================================================================
#
# Estimate the key length of a Vigenere-encrypted ciphertext using
# Index of Coincidence analysis.
#
# For each candidate key length k (2..max_length):
#   1. Split ciphertext into k groups (every k-th letter).
#   2. Compute IC of each group.
#   3. Average the ICs.
# The key length with the highest average IC is most likely correct.

sub find_key_length {
    my ($ciphertext, $max_length) = @_;
    $max_length //= 20;

    # Extract only alphabetic characters
    my $alpha_only = $ciphertext;
    $alpha_only =~ s/[^A-Za-z]//g;
    my $n = length($alpha_only);

    return 1 if $n < 2;

    my $best_length = 1;
    my $best_ic = -1;

    my $limit = $max_length < int($n / 2) ? $max_length : int($n / 2);

    for my $k (2 .. $limit) {
        my $ic_sum = 0;

        for my $j (0 .. $k - 1) {
            # Build the group: every k-th character starting at position j
            my $group = "";
            my $pos = $j;
            while ($pos < $n) {
                $group .= substr($alpha_only, $pos, 1);
                $pos += $k;
            }
            $ic_sum += _index_of_coincidence($group);
        }

        my $avg_ic = $ic_sum / $k;
        if ($avg_ic > $best_ic) {
            $best_ic = $avg_ic;
            $best_length = $k;
        }
    }

    return $best_length;
}

# ============================================================================
# _chi_squared(\@counts, $total) -> $chi2
# ============================================================================
#
# Chi-squared statistic against English letter frequencies.
# Lower values mean a better fit to English.

sub _chi_squared {
    my ($counts, $total) = @_;
    my $chi2 = 0;

    for my $i (0 .. 25) {
        my $expected = $total * $ENGLISH_FREQ[$i];
        if ($expected > 0) {
            my $diff = $counts->[$i] - $expected;
            $chi2 += ($diff * $diff) / $expected;
        }
    }

    return $chi2;
}

# ============================================================================
# find_key($ciphertext, $key_length) -> $key
# ============================================================================
#
# Given a ciphertext and known key length, find the key by chi-squared
# analysis on each position.
#
# For each key position (0..key_length-1):
#   1. Extract the group of letters at that position.
#   2. Try all 26 possible shifts.
#   3. The shift with the lowest chi-squared is the key letter.

sub find_key {
    my ($ciphertext, $key_length) = @_;

    # Extract only alphabetic characters, normalized to 0-25
    my @alpha;
    for my $ch (split //, $ciphertext) {
        if ($ch =~ /[A-Za-z]/) {
            push @alpha, ord(uc($ch)) - ord('A');
        }
    }
    my $n = scalar @alpha;

    my $key = "";

    for my $pos (0 .. $key_length - 1) {
        # Gather letters at this key position
        my @group;
        my $idx = $pos;
        while ($idx < $n) {
            push @group, $alpha[$idx];
            $idx += $key_length;
        }

        my $group_size = scalar @group;
        if ($group_size == 0) {
            $key .= 'A';
            next;
        }

        # Try all 26 shifts, pick the one with lowest chi-squared
        my $best_shift = 0;
        my $best_chi2 = 1e30;

        for my $shift (0 .. 25) {
            my @counts = (0) x 26;
            for my $val (@group) {
                my $decrypted = ($val - $shift + 26) % 26;
                $counts[$decrypted]++;
            }

            my $chi2 = _chi_squared(\@counts, $group_size);
            if ($chi2 < $best_chi2) {
                $best_chi2 = $chi2;
                $best_shift = $shift;
            }
        }

        $key .= chr(ord('A') + $best_shift);
    }

    return $key;
}

# ============================================================================
# break_cipher($ciphertext) -> ($key, $plaintext)
# ============================================================================
#
# Automatic Vigenere cipher break. Combines find_key_length and find_key
# to recover the key and plaintext without any prior knowledge.
#
# Returns a list of two values: (key, plaintext).

sub break_cipher {
    my ($ciphertext) = @_;

    my $key_length = find_key_length($ciphertext);
    my $key = find_key($ciphertext, $key_length);
    my $plaintext = decrypt($ciphertext, $key);

    return ($key, $plaintext);
}

1;
