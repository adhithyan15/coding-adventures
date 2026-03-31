use strict;
use warnings;
use Test2::V0;

use CodingAdventures::CaesarCipher;

# ============================================================================
# Cryptanalysis Tests
#
# These tests verify the brute_force and frequency_analysis functions.
# ============================================================================

# --- Brute Force ------------------------------------------------------------

subtest 'brute_force returns 25 results' => sub {
    my @results = CodingAdventures::CaesarCipher::brute_force("KHOOR");
    is(scalar @results, 25, 'brute force produces exactly 25 results (shifts 1-25)');
};

subtest 'brute_force result structure' => sub {
    my @results = CodingAdventures::CaesarCipher::brute_force("KHOOR");

    # Each result should be a hashref with 'shift' and 'text' keys.
    for my $result (@results) {
        ok(exists $result->{shift}, "result has 'shift' key");
        ok(exists $result->{text},  "result has 'text' key");
    }
};

subtest 'brute_force shifts are 1 through 25' => sub {
    my @results = CodingAdventures::CaesarCipher::brute_force("TEST");
    my @shifts = map { $_->{shift} } @results;
    is(\@shifts, [1 .. 25], 'shifts cover 1 through 25 in order');
};

subtest 'brute_force contains correct decryption' => sub {
    # "KHOOR" was encrypted with shift 3, so shift 3 should yield "HELLO".
    my @results = CodingAdventures::CaesarCipher::brute_force("KHOOR");

    # Shift 3 is at index 2 (since results start at shift 1).
    is($results[2]->{shift}, 3, 'third result has shift 3');
    is($results[2]->{text}, "HELLO", 'shift 3 decrypts KHOOR to HELLO');
};

subtest 'brute_force with lowercase' => sub {
    my @results = CodingAdventures::CaesarCipher::brute_force("khoor");
    is($results[2]->{text}, "hello", 'brute force works with lowercase');
};

subtest 'brute_force with mixed content' => sub {
    my @results = CodingAdventures::CaesarCipher::brute_force("Khoor, Zruog!");
    is($results[2]->{text}, "Hello, World!", 'brute force preserves non-alpha chars');
};

# --- Frequency Analysis -----------------------------------------------------

subtest 'frequency_analysis on known English text' => sub {
    # A reasonably long English text encrypted with shift 7.
    my $plaintext = "the quick brown fox jumps over the lazy dog and "
                  . "the five boxing wizards jump quickly over fences "
                  . "while the early bird catches the worm every morning";
    my $shift = 7;
    my $ciphertext = CodingAdventures::CaesarCipher::encrypt($plaintext, $shift);

    my $detected = CodingAdventures::CaesarCipher::frequency_analysis($ciphertext);
    is($detected, $shift, "frequency analysis correctly detects shift of $shift");
};

subtest 'frequency_analysis on shift 3 text' => sub {
    my $plaintext = "in the beginning there was nothing and then there was "
                  . "something and that something became everything we know "
                  . "the universe expanded from a single point of light";
    my $shift = 3;
    my $ciphertext = CodingAdventures::CaesarCipher::encrypt($plaintext, $shift);

    my $detected = CodingAdventures::CaesarCipher::frequency_analysis($ciphertext);
    is($detected, $shift, "frequency analysis correctly detects shift of $shift");
};

subtest 'frequency_analysis on shift 13 (ROT13)' => sub {
    my $plaintext = "the art of writing is the art of discovering what you "
                  . "believe and the essence of style is the sound it makes "
                  . "in the readers mind where clarity and rhythm combine to "
                  . "create something greater than the sum of its parts";
    my $shift = 13;
    my $ciphertext = CodingAdventures::CaesarCipher::encrypt($plaintext, $shift);

    my $detected = CodingAdventures::CaesarCipher::frequency_analysis($ciphertext);
    is($detected, $shift, "frequency analysis correctly detects shift of $shift");
};

subtest 'frequency_analysis on empty string returns 0' => sub {
    my $detected = CodingAdventures::CaesarCipher::frequency_analysis("");
    is($detected, 0, 'empty string yields shift 0');
};

subtest 'frequency_analysis on non-alpha string returns 0' => sub {
    my $detected = CodingAdventures::CaesarCipher::frequency_analysis("12345!@#\$%");
    is($detected, 0, 'string with no letters yields shift 0');
};

subtest 'frequency_analysis on shift 0 text' => sub {
    my $plaintext = "the quick brown fox jumps over the lazy dog "
                  . "every good boy does fine all cows eat grass";
    my $detected = CodingAdventures::CaesarCipher::frequency_analysis($plaintext);
    is($detected, 0, 'unencrypted English text yields shift 0');
};

done_testing;
