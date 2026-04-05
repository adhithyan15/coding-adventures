#!/usr/bin/env perl
# hash-breaker — Demonstrating why MD5 is cryptographically broken.
#
# Three attacks against MD5:
#   1. Known Collision Pairs (Wang & Yu, 2004)
#   2. Length Extension Attack (forge hash without secret)
#   3. Birthday Attack on truncated hash (birthday paradox)

use strict;
use warnings;
use POSIX qw(floor);

# Add our MD5 library to the include path
use lib "../../../packages/perl/md5/lib";
use CodingAdventures::Md5;

# ============================================================================
# Utility functions
# ============================================================================

sub hex_to_bytes {
    my ($hex) = @_;
    return pack("H*", $hex);
}

sub bytes_to_hex {
    my ($bytes) = @_;
    return unpack("H*", $bytes);
}

sub hex_dump {
    my ($data) = @_;
    my @bytes = unpack("C*", $data);
    my @lines;
    for (my $i = 0; $i < scalar(@bytes); $i += 16) {
        my @row = @bytes[$i .. ($i + 15 < $#bytes ? $i + 15 : $#bytes)];
        push @lines, "  " . join("", map { sprintf("%02x", $_) } @row);
    }
    return join("\n", @lines);
}

# ============================================================================
# ATTACK 1: Known MD5 Collision Pairs (Wang & Yu, 2004)
# ============================================================================

my $collision_a_hex =
    "d131dd02c5e6eec4693d9a0698aff95c" .
    "2fcab58712467eab4004583eb8fb7f89" .
    "55ad340609f4b30283e488832571415a" .
    "085125e8f7cdc99fd91dbdf280373c5b" .
    "d8823e3156348f5bae6dacd436c919c6" .
    "dd53e2b487da03fd02396306d248cda0" .
    "e99f33420f577ee8ce54b67080a80d1e" .
    "c69821bcb6a8839396f9652b6ff72a70";

my $collision_b_hex =
    "d131dd02c5e6eec4693d9a0698aff95c" .
    "2fcab50712467eab4004583eb8fb7f89" .
    "55ad340609f4b30283e4888325f1415a" .
    "085125e8f7cdc99fd91dbd7280373c5b" .
    "d8823e3156348f5bae6dacd436c919c6" .
    "dd53e23487da03fd02396306d248cda0" .
    "e99f33420f577ee8ce54b67080280d1e" .
    "c69821bcb6a8839396f965ab6ff72a70";

sub attack_1 {
    print "=" x 72, "\n";
    print "ATTACK 1: Known MD5 Collision Pair (Wang & Yu, 2004)\n";
    print "=" x 72, "\n";
    print "\n";
    print "Two different 128-byte messages that produce the SAME MD5 hash.\n";
    print "This was the breakthrough that proved MD5 is broken for security.\n";
    print "\n";

    my $bytes_a = hex_to_bytes($collision_a_hex);
    my $bytes_b = hex_to_bytes($collision_b_hex);

    print "Block A (hex):\n";
    print hex_dump($bytes_a), "\n\n";
    print "Block B (hex):\n";
    print hex_dump($bytes_b), "\n\n";

    # Show byte differences
    my @ba = unpack("C*", $bytes_a);
    my @bb = unpack("C*", $bytes_b);
    my @diffs;
    for my $i (0 .. $#ba) {
        push @diffs, $i if $ba[$i] != $bb[$i];
    }
    printf "Blocks differ at %d byte positions: [%s]\n", scalar(@diffs), join(", ", @diffs);
    for my $pos (@diffs) {
        printf "  Byte %d: A=0x%02x  B=0x%02x\n", $pos, $ba[$pos], $bb[$pos];
    }
    print "\n";

    my $hash_a = CodingAdventures::Md5::hex($bytes_a);
    my $hash_b = CodingAdventures::Md5::hex($bytes_b);
    print "MD5(A) = $hash_a\n";
    print "MD5(B) = $hash_b\n";
    my $match = $hash_a eq $hash_b ? "YES — COLLISION!" : "No (unexpected)";
    print "Match?   $match\n";
    print "\n";
    print "Lesson: MD5 collisions are REAL. Never use MD5 for integrity or auth.\n";
    print "\n";
}

# ============================================================================
# ATTACK 2: Length Extension Attack
# ============================================================================

# MD5 T-table constants
my @T_TABLE;
for my $i (0 .. 63) {
    $T_TABLE[$i] = floor(abs(sin($i + 1)) * 2**32) & 0xFFFFFFFF;
}

# MD5 shifts
my @SHIFTS = (
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22,
    5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20,
    4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23,
    6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21
);

sub left_rotate {
    my ($x, $n) = @_;
    return (($x << $n) | ($x >> (32 - $n))) & 0xFFFFFFFF;
}

# Inline MD5 compression for length extension attack
sub md5_compress {
    my ($state, $block) = @_;
    my @m = unpack("V16", $block);
    my ($a, $b, $c, $d) = @$state;
    my ($a0, $b0, $c0, $d0) = ($a, $b, $c, $d);

    for my $i (0 .. 63) {
        my ($f, $g);
        if ($i < 16) {
            $f = ($b & $c) | (~$b & $d);
            $g = $i;
        } elsif ($i < 32) {
            $f = ($d & $b) | (~$d & $c);
            $g = (5 * $i + 1) % 16;
        } elsif ($i < 48) {
            $f = $b ^ $c ^ $d;
            $g = (3 * $i + 5) % 16;
        } else {
            $f = $c ^ ($b | ~$d);
            $g = (7 * $i) % 16;
        }
        $f &= 0xFFFFFFFF;

        my $temp = $d;
        $d = $c;
        $c = $b;
        $b = ($b + left_rotate(($a + $f + $T_TABLE[$i] + $m[$g]) & 0xFFFFFFFF, $SHIFTS[$i])) & 0xFFFFFFFF;
        $a = $temp;
    }

    return [($a0 + $a) & 0xFFFFFFFF, ($b0 + $b) & 0xFFFFFFFF,
            ($c0 + $c) & 0xFFFFFFFF, ($d0 + $d) & 0xFFFFFFFF];
}

sub md5_padding {
    my ($message_len) = @_;
    my $remainder = $message_len % 64;
    my $pad_len = (55 - $remainder) % 64;
    my $padding = chr(0x80) . ("\x00" x $pad_len);
    my $bit_len = $message_len * 8;
    my $lo = $bit_len & 0xFFFFFFFF;
    my $hi = int($bit_len / 2**32) & 0xFFFFFFFF;
    $padding .= pack("VV", $lo, $hi);
    return $padding;
}

sub attack_2 {
    print "=" x 72, "\n";
    print "ATTACK 2: Length Extension Attack\n";
    print "=" x 72, "\n";
    print "\n";
    print "Given md5(secret + message) and len(secret + message), we can forge\n";
    print "md5(secret + message + padding + evil_data) WITHOUT knowing the secret!\n";
    print "\n";

    my $secret = "supersecretkey!!";
    my $message = "amount=100&to=alice";
    my $original_data = $secret . $message;
    my $original_hash_bytes = CodingAdventures::Md5::digest($original_data);
    my $original_hash = pack("C*", @$original_hash_bytes);
    my $original_hex = bytes_to_hex($original_hash);

    print "Secret (unknown to attacker): \"$secret\"\n";
    print "Message:                      \"$message\"\n";
    print "MAC = md5(secret || message): $original_hex\n";
    printf "Length of (secret || message): %d bytes\n", length($original_data);
    print "\n";

    my $evil_data = "&amount=1000000&to=mallory";
    print "Evil data to append: \"$evil_data\"\n";
    print "\n";

    # Step 1: Extract state from hash
    my ($a, $b, $c, $d) = unpack("V4", $original_hash);
    print "Step 1: Extract MD5 internal state from the hash\n";
    printf "  A = 0x%08x, B = 0x%08x, C = 0x%08x, D = 0x%08x\n", $a, $b, $c, $d;
    print "\n";

    # Step 2: Compute padding
    my $padding = md5_padding(length($original_data));
    print "Step 2: Compute MD5 padding for the original message\n";
    printf "  Padding (%d bytes): %s\n", length($padding), bytes_to_hex($padding);
    print "\n";

    my $processed_len = length($original_data) + length($padding);
    print "Step 3: Total bytes processed so far: $processed_len\n";
    print "\n";

    # Step 4: Forge
    my $forged_input = $evil_data . md5_padding($processed_len + length($evil_data));
    my $state = [$a, $b, $c, $d];
    for (my $i = 0; $i + 64 <= length($forged_input); $i += 64) {
        my $block = substr($forged_input, $i, 64);
        $state = md5_compress($state, $block);
    }
    my $forged_hash = pack("V4", @$state);
    my $forged_hex = bytes_to_hex($forged_hash);

    print "Step 4: Initialize hasher with extracted state, feed evil_data\n";
    print "  Forged hash: $forged_hex\n";
    print "\n";

    # Step 5: Verify
    my $actual_full = $original_data . $padding . $evil_data;
    my $actual_hex = CodingAdventures::Md5::hex($actual_full);

    print "Step 5: Verify — compute actual md5(secret || message || padding || evil_data)\n";
    print "  Actual hash: $actual_hex\n";
    my $match = $forged_hex eq $actual_hex ? "YES — FORGED!" : "No (bug)";
    print "  Match?       $match\n";
    print "\n";
    print "The attacker forged a valid MAC without knowing the secret!\n";
    print "\n";
    print "Why HMAC fixes this:\n";
    print "  HMAC = md5(key XOR opad || md5(key XOR ipad || message))\n";
    print "  The outer hash prevents length extension because the attacker\n";
    print "  cannot extend past the outer md5() boundary.\n";
    print "\n";
}

# ============================================================================
# ATTACK 3: Birthday Attack (Truncated Hash)
# ============================================================================

sub attack_3 {
    print "=" x 72, "\n";
    print "ATTACK 3: Birthday Attack on Truncated MD5 (32-bit)\n";
    print "=" x 72, "\n";
    print "\n";
    print "The birthday paradox: with N possible hash values, expect a collision\n";
    print "after ~sqrt(N) random inputs. For 32-bit hash: sqrt(2^32) = 2^16 = 65536.\n";
    print "\n";

    # Deterministic xorshift32 PRNG
    my $rng_state = 42;
    my $xorshift32 = sub {
        $rng_state ^= ($rng_state << 13) & 0xFFFFFFFF;
        $rng_state ^= ($rng_state >> 17);
        $rng_state ^= ($rng_state << 5) & 0xFFFFFFFF;
        $rng_state &= 0xFFFFFFFF;
        return $rng_state;
    };

    my %seen;
    my $attempts = 0;

    while (1) {
        $attempts++;
        # Generate random 8-byte message
        my @msg_bytes;
        for (1 .. 8) {
            push @msg_bytes, $xorshift32->() & 0xFF;
        }
        my $msg = pack("C*", @msg_bytes);
        my $msg_hex = bytes_to_hex($msg);

        my $hash_bytes = CodingAdventures::Md5::digest($msg);
        my $truncated_hex = sprintf("%02x%02x%02x%02x",
            $hash_bytes->[0], $hash_bytes->[1], $hash_bytes->[2], $hash_bytes->[3]);

        if (exists $seen{$truncated_hex}) {
            my $other_hex = $seen{$truncated_hex};
            if ($other_hex ne $msg_hex) {
                printf "COLLISION FOUND after %d attempts!\n\n", $attempts;
                print "  Message 1: $other_hex\n";
                print "  Message 2: $msg_hex\n";
                print "  Truncated MD5 (4 bytes): $truncated_hex\n";
                my $other = hex_to_bytes($other_hex);
                print "  Full MD5 of msg1: " . CodingAdventures::Md5::hex($other) . "\n";
                print "  Full MD5 of msg2: " . CodingAdventures::Md5::hex($msg) . "\n";
                print "\n";
                printf "  Expected ~65536 attempts (2^16), got %d\n", $attempts;
                printf "  Ratio: %.2fx the theoretical expectation\n", $attempts / 65536;
                last;
            }
        } else {
            $seen{$truncated_hex} = $msg_hex;
        }
    }

    print "\n";
    print "This is a GENERIC attack — it works against any hash function.\n";
    print "The defense is a longer hash: SHA-256 has 2^128 birthday bound,\n";
    print "while MD5 has only 2^64 (and dedicated attacks are even faster).\n";
    print "\n";
}

# ============================================================================
# Main
# ============================================================================

print "\n";
print "======================================================================\n";
print "           MD5 HASH BREAKER — Why MD5 Is Broken\n";
print "======================================================================\n";
print "  Three attacks showing MD5 must NEVER be used for security:\n";
print "    1. Known collision pairs (Wang & Yu, 2004)\n";
print "    2. Length extension attack (forge MAC without secret)\n";
print "    3. Birthday attack on truncated hash (birthday paradox)\n";
print "======================================================================\n";
print "\n";

attack_1();
attack_2();
attack_3();

print "=" x 72, "\n";
print "CONCLUSION\n";
print "=" x 72, "\n";
print "\n";
print "MD5 is broken in three distinct ways:\n";
print "  1. COLLISION RESISTANCE: known pairs exist (and can be generated)\n";
print "  2. LENGTH EXTENSION: Merkle-Damgard structure leaks internal state\n";
print "  3. BIRTHDAY BOUND: only 2^64 (and dedicated attacks beat even that)\n";
print "\n";
print "Use SHA-256 or SHA-3 for security. Use HMAC (not raw hash) for MACs.\n";
print "\n";
