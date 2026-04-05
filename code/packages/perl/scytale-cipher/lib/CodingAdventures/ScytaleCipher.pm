package CodingAdventures::ScytaleCipher;

# ============================================================================
# CodingAdventures::ScytaleCipher
# ============================================================================
#
# The Scytale (pronounced "SKIT-ah-lee") cipher is a *transposition* cipher
# from ancient Sparta (~700 BCE). Unlike substitution ciphers (Caesar, Atbash)
# which replace characters, the Scytale rearranges character positions using
# a columnar transposition.
#
# How Encryption Works
# --------------------
#
# 1. Write text row-by-row into a grid with `key` columns.
# 2. Pad the last row with spaces if needed.
# 3. Read column-by-column to produce ciphertext.
#
# Example: encrypt("HELLO WORLD", 3)
#
#     Grid (4 rows x 3 cols):
#         H E L
#         L O ' '
#         W O R
#         L D ' '
#
#     Columns: HLWL + EOOD + L R  = "HLWLEOODL R "
#
# How Decryption Works
# --------------------
#
# 1. Calculate rows = ceil(len / key).
# 2. Write ciphertext column-by-column.
# 3. Read row-by-row and strip trailing padding spaces.

use strict;
use warnings;
use POSIX qw(ceil);
use Exporter 'import';

our $VERSION = '0.1.0';
our @EXPORT_OK = qw(encrypt decrypt brute_force);

# Encrypt text using the Scytale transposition cipher.
#
# Arguments:
#   $text - The plaintext string
#   $key  - Number of columns (>= 2, <= length of text)
#
# Returns the transposed ciphertext.
sub encrypt {
    my ($text, $key) = @_;
    return "" if $text eq "";

    my $n = length($text);
    die "Key must be >= 2, got $key\n" if $key < 2;
    die "Key must be <= text length ($n), got $key\n" if $key > $n;

    # Calculate grid dimensions and pad
    my $num_rows = ceil($n / $key);
    my $padded_len = $num_rows * $key;
    my $padded = $text . (" " x ($padded_len - $n));

    # Read column-by-column
    my $result = "";
    for my $col (0 .. $key - 1) {
        for my $row (0 .. $num_rows - 1) {
            $result .= substr($padded, $row * $key + $col, 1);
        }
    }

    return $result;
}

# Decrypt ciphertext encrypted with the Scytale cipher.
# Trailing padding spaces are stripped.
#
# Arguments:
#   $text - The ciphertext string
#   $key  - Number of columns used during encryption
#
# Returns the decrypted plaintext.
sub decrypt {
    my ($text, $key) = @_;
    return "" if $text eq "";

    my $n = length($text);
    die "Key must be >= 2, got $key\n" if $key < 2;
    die "Key must be <= text length ($n), got $key\n" if $key > $n;

    my $num_rows = ceil($n / $key);

    # Handle uneven grids (when n % key != 0, e.g. during brute-force)
    my $full_cols = $n % $key == 0 ? $key : $n % $key;

    # Compute column start indices and lengths
    my @col_starts;
    my @col_lens;
    my $offset = 0;
    for my $c (0 .. $key - 1) {
        push @col_starts, $offset;
        my $col_len = ($n % $key == 0 || $c < $full_cols) ? $num_rows : $num_rows - 1;
        push @col_lens, $col_len;
        $offset += $col_len;
    }

    # Read row-by-row
    my $result = "";
    for my $row (0 .. $num_rows - 1) {
        for my $col (0 .. $key - 1) {
            if ($row < $col_lens[$col]) {
                $result .= substr($text, $col_starts[$col] + $row, 1);
            }
        }
    }

    # Strip trailing padding spaces
    $result =~ s/\s+$//;
    return $result;
}

# Try all possible keys and return decryption results.
#
# Arguments:
#   $text - The ciphertext to brute-force
#
# Returns an array of hashrefs [{key => N, text => "..."}, ...]
sub brute_force {
    my ($text) = @_;
    my $n = length($text);
    return () if $n < 4;

    my $max_key = int($n / 2);
    my @results;

    for my $candidate_key (2 .. $max_key) {
        push @results, {
            key  => $candidate_key,
            text => decrypt($text, $candidate_key),
        };
    }

    return @results;
}

1;
