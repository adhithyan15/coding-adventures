package CodingAdventures::AtbashCipher;

# ============================================================================
# CodingAdventures::AtbashCipher
# ============================================================================
#
# The Atbash cipher: a fixed reverse-alphabet substitution cipher.
#
# What is the Atbash Cipher?
# --------------------------
#
# The Atbash cipher is one of the oldest known substitution ciphers,
# originally used with the Hebrew alphabet. The name "Atbash" comes from
# the first, last, second, and second-to-last letters of the Hebrew
# alphabet: Aleph-Tav-Beth-Shin.
#
# The cipher reverses the alphabet:
#
#     Plain:  A B C D E F G H I J K L M N O P Q R S T U V W X Y Z
#     Cipher: Z Y X W V U T S R Q P O N M L K J I H G F E D C B A
#
# The Formula
# -----------
#
# Given a letter at position p (where A=0, B=1, ..., Z=25):
#
#     encrypted_position = 25 - p
#
# For example, 'H' is at position 7: 25 - 7 = 18, which is 'S'.
#
# Self-Inverse Property
# ---------------------
#
# f(f(x)) = 25 - (25 - x) = x
#
# This means encrypt() and decrypt() are the same operation.
#
# Usage:
#
#   use CodingAdventures::AtbashCipher;
#   my $encrypted = CodingAdventures::AtbashCipher::encrypt("HELLO");
#   # $encrypted is "SVOOL"
#
# ============================================================================

use strict;
use warnings;
use Exporter 'import';

our $VERSION = '0.01';
our @EXPORT_OK = qw(encrypt decrypt);

# encrypt($text) -> $encrypted
#
# Encrypt text using the Atbash cipher.
#
# Each letter is replaced by its reverse in the alphabet (A<->Z, B<->Y, etc.).
# Non-alphabetic characters pass through unchanged. Case is preserved.
#
# Because the Atbash cipher is self-inverse, this function is identical
# to decrypt(). Both are provided for API clarity.
#
# The implementation uses Perl's tr/// (transliterate) operator, which is
# the most idiomatic and efficient way to do character-by-character
# substitution in Perl. tr/SEARCHLIST/REPLACELIST/ replaces each character
# in the search list with the corresponding character in the replacement list.
sub encrypt {
    my ($text) = @_;

    # Make a copy so we don't modify the original.
    my $result = $text;

    # tr/// does character-by-character substitution. We map:
    #   A-Z to Z-A (uppercase reversal)
    #   a-z to z-a (lowercase reversal)
    #
    # The tr operator with ranges like A-Z means all characters from A to Z.
    # When we write Z-A as the replacement, Perl understands this as the
    # reversed sequence: Z, Y, X, W, ..., B, A.
    #
    # Characters not in the search list pass through unchanged, which
    # gives us automatic non-alpha passthrough.
    $result =~ tr/ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz/ZYXWVUTSRQPONMLKJIHGFEDCBAzyxwvutsrqponmlkjihgfedcba/;

    return $result;
}

# decrypt($text) -> $decrypted
#
# Decrypt text using the Atbash cipher.
#
# Because the Atbash cipher is self-inverse (applying it twice returns
# the original), decryption is identical to encryption. This function
# exists for API clarity.
sub decrypt {
    my ($text) = @_;

    # Decryption IS encryption for Atbash.
    # Proof: f(f(x)) = 25 - (25 - x) = x
    return encrypt($text);
}

1;

__END__

=head1 NAME

CodingAdventures::AtbashCipher - Atbash cipher: fixed reverse-alphabet substitution, self-inverse

=head1 SYNOPSIS

    use CodingAdventures::AtbashCipher qw(encrypt decrypt);

    my $ciphertext = encrypt("HELLO");        # "SVOOL"
    my $plaintext  = decrypt("SVOOL");        # "HELLO"
    my $mixed      = encrypt("Hello, World! 123"); # "Svool, Dliow! 123"

=head1 DESCRIPTION

The Atbash cipher reverses the alphabet: A maps to Z, B maps to Y, C maps
to X, and so on. The formula is: encrypted_position = 25 - original_position.

The cipher is self-inverse: encrypt(encrypt(text)) returns the original text.
This means decrypt() is identical to encrypt().

Case is preserved. Non-alphabetic characters pass through unchanged.

=head1 FUNCTIONS

=head2 encrypt($text)

Encrypt text using the Atbash cipher.

=head2 decrypt($text)

Decrypt text using the Atbash cipher (identical to encrypt).

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
