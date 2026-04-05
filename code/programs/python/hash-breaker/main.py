#!/usr/bin/env python3
"""hash-breaker — Demonstrating why MD5 is cryptographically broken.

This program runs three attacks against MD5 to show, in concrete terms, why
you must never use MD5 for security:

  1. Known Collision Pairs — two different byte sequences with the same MD5
  2. Length Extension Attack — forge a valid hash without knowing the secret
  3. Birthday Attack — find a collision on a truncated hash via birthday paradox

Each attack prints educational output explaining the cryptographic concept.
"""

import os
import struct
import random

# ── Import our own MD5 package ──────────────────────────────────────────────
# This is the coding-adventures MD5 implementation, not Python's hashlib.
from coding_adventures_md5 import md5, md5_hex, MD5


# ============================================================================
# ATTACK 1: Known MD5 Collision Pairs (Wang & Yu, 2004)
# ============================================================================
#
# In 2004, Xiaoyun Wang and Hongbo Yu published the first practical collision
# attack on MD5.  They found two 128-byte messages (two 512-bit blocks each)
# that produce the SAME MD5 hash despite differing in specific bit positions.
#
# A collision means:  md5(A) == md5(B)  but  A != B
#
# This destroys MD5's usefulness for digital signatures: an attacker could get
# a signature on document A, then substitute document B (same hash) — the
# signature would still verify.
#
# The two blocks below are the canonical Wang/Yu collision pair.  They differ
# in only a handful of bytes (specifically at offsets that affect the internal
# state differences the attack exploits).

COLLISION_A = bytes.fromhex(
    "d131dd02c5e6eec4693d9a0698aff95c"
    "2fcab58712467eab4004583eb8fb7f89"
    "55ad340609f4b30283e488832571415a"
    "085125e8f7cdc99fd91dbdf280373c5b"
    "d8823e3156348f5bae6dacd436c919c6"
    "dd53e2b487da03fd02396306d248cda0"
    "e99f33420f577ee8ce54b67080a80d1e"
    "c69821bcb6a8839396f9652b6ff72a70"
)

COLLISION_B = bytes.fromhex(
    "d131dd02c5e6eec4693d9a0698aff95c"
    "2fcab50712467eab4004583eb8fb7f89"
    "55ad340609f4b30283e4888325f1415a"
    "085125e8f7cdc99fd91dbd7280373c5b"
    "d8823e3156348f5bae6dacd436c919c6"
    "dd53e23487da03fd02396306d248cda0"
    "e99f33420f577ee8ce54b67080280d1e"
    "c69821bcb6a8839396f965ab6ff72a70"
)


def attack_1_known_collision() -> None:
    """Demonstrate MD5 collision using known Wang/Yu pair."""
    print("=" * 72)
    print("ATTACK 1: Known MD5 Collision Pair (Wang & Yu, 2004)")
    print("=" * 72)
    print()
    print("Two different 128-byte messages that produce the SAME MD5 hash.")
    print("This was the breakthrough that proved MD5 is broken for security.")
    print()

    # Show the two blocks differ
    print("Block A (hex):")
    for i in range(0, len(COLLISION_A), 16):
        print(f"  {COLLISION_A[i:i+16].hex()}")
    print()

    print("Block B (hex):")
    for i in range(0, len(COLLISION_B), 16):
        print(f"  {COLLISION_B[i:i+16].hex()}")
    print()

    # Show the byte differences
    diffs = [i for i in range(len(COLLISION_A)) if COLLISION_A[i] != COLLISION_B[i]]
    print(f"Blocks differ at {len(diffs)} byte positions: {diffs}")
    for pos in diffs:
        print(f"  Byte {pos}: A=0x{COLLISION_A[pos]:02x}  B=0x{COLLISION_B[pos]:02x}")
    print()

    # Compute MD5 of both
    hash_a = md5_hex(COLLISION_A)
    hash_b = md5_hex(COLLISION_B)
    print(f"MD5(A) = {hash_a}")
    print(f"MD5(B) = {hash_b}")
    print(f"Match?   {'YES — COLLISION!' if hash_a == hash_b else 'No (unexpected)'}")
    print()

    # Show that SHA-1 (a stronger hash) distinguishes them
    # We'll use our own SHA-1 if available, otherwise note it
    try:
        from coding_adventures_sha1 import sha1_hex
        sha_a = sha1_hex(COLLISION_A)
        sha_b = sha1_hex(COLLISION_B)
        print(f"SHA-1(A) = {sha_a}")
        print(f"SHA-1(B) = {sha_b}")
        print(f"SHA-1 distinguishes them? {'YES' if sha_a != sha_b else 'No'}")
    except ImportError:
        print("(SHA-1 package not available — but any stronger hash would")
        print(" produce different digests for these two blocks.)")
    print()
    print("Lesson: MD5 collisions are REAL. Never use MD5 for integrity or auth.")
    print()


# ============================================================================
# ATTACK 2: Length Extension Attack
# ============================================================================
#
# MD5 (and all Merkle-Damgard hashes like SHA-1, SHA-256) are vulnerable to
# length extension attacks.  Given:
#
#   - hash = md5(secret || message)
#   - len(secret || message)     (but NOT the secret itself)
#
# An attacker can compute:
#
#   md5(secret || message || padding || evil_data)
#
# WITHOUT knowing the secret!
#
# How it works:
#   1. The MD5 hash IS the internal state after processing all blocks.
#   2. We can extract the four 32-bit words (A, B, C, D) from the hash.
#   3. We compute what the padding would be for the original message.
#   4. We initialize a new MD5 hasher with the extracted state and a fake
#      length, then feed in our evil_data.
#   5. The resulting hash matches md5(secret || message || padding || evil_data).
#
# This is why naive MAC = md5(secret || message) is INSECURE.
# HMAC fixes this by using md5(key XOR opad || md5(key XOR ipad || message)).

def md5_padding(message_len: int) -> bytes:
    """Compute the MD5 padding that would be appended to a message of the given length.

    MD5 padding:
      1. Append 0x80 (a single 1-bit followed by zeros)
      2. Append zero bytes until total length ≡ 56 mod 64
      3. Append original length in bits as 64-bit little-endian integer
    """
    # Number of bytes already in the last block
    remainder = message_len % 64
    # We need (56 - remainder - 1) zero bytes after the 0x80 byte
    # If remainder >= 56, we need an extra block
    pad_len = (55 - remainder) % 64
    padding = b"\x80" + b"\x00" * pad_len
    # Append the original bit length as a 64-bit little-endian integer
    bit_len = message_len * 8
    padding += struct.pack("<Q", bit_len)
    return padding


def attack_2_length_extension() -> None:
    """Demonstrate MD5 length extension attack."""
    print("=" * 72)
    print("ATTACK 2: Length Extension Attack")
    print("=" * 72)
    print()
    print("Given md5(secret + message) and len(secret + message), we can forge")
    print("md5(secret + message + padding + evil_data) WITHOUT knowing the secret!")
    print()

    # Setup: server computes MAC = md5(secret || message)
    secret = b"supersecretkey!!"  # 16 bytes — attacker does NOT know this
    message = b"amount=100&to=alice"
    original_data = secret + message
    original_hash = md5(original_data)
    original_hex = original_hash.hex()

    print("Secret (unknown to attacker): <redacted 16-byte demo secret>")
    print(f"Message:                      {message!r}")
    print(f"MAC = md5(secret || message): {original_hex}")
    print(f"Length of (secret || message): {len(original_data)} bytes")
    print()

    # Attacker's goal: forge md5(secret || message || padding || evil_data)
    evil_data = b"&amount=1000000&to=mallory"
    print(f"Evil data to append: {evil_data!r}")
    print()

    # Step 1: Extract internal state from the hash
    # The MD5 hash IS the four 32-bit state words in little-endian order.
    a, b, c, d = struct.unpack("<4I", original_hash)
    print("Step 1: Extract MD5 internal state from the hash")
    print(f"  A = 0x{a:08x}, B = 0x{b:08x}, C = 0x{c:08x}, D = 0x{d:08x}")
    print()

    # Step 2: Compute the padding that was applied to the original message
    padding = md5_padding(len(original_data))
    print("Step 2: Compute MD5 padding for the original message")
    print(f"  Padding ({len(padding)} bytes): {padding.hex()}")
    print()

    # Step 3: The total processed length so far is len(original_data + padding)
    processed_len = len(original_data) + len(padding)
    print(f"Step 3: Total bytes processed so far: {processed_len}")
    print()

    # Step 4: Build a new hasher with the extracted state
    # We need to set the internal state to (a, b, c, d) and the byte count
    # to processed_len, then update with evil_data.
    #
    # Since our MD5 class uses a streaming API, we'll manually construct
    # the forged hash by computing what the compression would produce.
    # We pad (evil_data) as if it's a continuation of a message of length
    # processed_len + len(evil_data).
    forged_input = evil_data + md5_padding(processed_len + len(evil_data))

    # We need to compress the evil_data blocks starting from the extracted state.
    # Import the internal compress function.
    from coding_adventures_md5 import _compress, _pad

    state = (a, b, c, d)
    # Process each 64-byte block of the padded evil data
    for i in range(0, len(forged_input), 64):
        block = forged_input[i:i + 64]
        if len(block) == 64:
            state = _compress(state, block)

    forged_hash = struct.pack("<4I", *state)
    forged_hex = forged_hash.hex()

    print("Step 4: Initialize hasher with extracted state, feed evil_data")
    print(f"  Forged hash: {forged_hex}")
    print()

    # Step 5: Verify — compute the actual hash of secret || message || padding || evil_data
    actual_full = secret + message + padding + evil_data
    actual_hex = md5_hex(actual_full)

    print("Step 5: Verify — compute actual md5(secret || message || padding || evil_data)")
    print(f"  Actual hash: {actual_hex}")
    print(f"  Match?       {'YES — FORGED!' if forged_hex == actual_hex else 'No (bug)'}")
    print()
    print("The attacker forged a valid MAC without knowing the secret!")
    print()
    print("Why HMAC fixes this:")
    print("  HMAC = md5(key XOR opad || md5(key XOR ipad || message))")
    print("  The outer hash prevents length extension because the attacker")
    print("  cannot extend past the outer md5() boundary.")
    print()


# ============================================================================
# ATTACK 3: Birthday Attack (Truncated Hash)
# ============================================================================
#
# The birthday paradox: in a group of 23 people, there's a >50% chance two
# share a birthday (out of 365 possible). More generally, with N possible
# values, you expect a collision after roughly sqrt(N) random samples.
#
# For a hash with B bits of output, the expected number of hashes before a
# collision is approximately 2^(B/2).
#
# Full MD5 has 128 bits → expect collision after ~2^64 hashes (too slow to demo).
# We truncate to 32 bits (4 bytes) → expect collision after ~2^16 = 65536 hashes.
#
# This attack is GENERIC — it works against ANY hash function. The only defense
# is a longer hash output. This is why SHA-256 (256 bits → 2^128 birthday bound)
# is preferred over MD5 (128 bits → 2^64 birthday bound, and MD5 has dedicated
# attacks that are even faster than birthday).

def attack_3_birthday() -> None:
    """Demonstrate birthday attack on truncated MD5."""
    print("=" * 72)
    print("ATTACK 3: Birthday Attack on Truncated MD5 (32-bit)")
    print("=" * 72)
    print()
    print("The birthday paradox: with N possible hash values, expect a collision")
    print("after ~sqrt(N) random inputs. For 32-bit hash: sqrt(2^32) = 2^16 = 65536.")
    print()

    seen: dict[bytes, bytes] = {}  # truncated_hash -> message
    attempts = 0

    random.seed(42)  # reproducible demo

    while True:
        attempts += 1
        # Generate a random 8-byte message
        msg = random.randbytes(8)
        # Compute MD5 and truncate to first 4 bytes (32 bits)
        full_hash = md5(msg)
        truncated = full_hash[:4]

        if truncated in seen:
            other_msg = seen[truncated]
            if other_msg != msg:  # sanity check: actual collision, not same message
                print(f"COLLISION FOUND after {attempts} attempts!")
                print()
                print(f"  Message 1: {other_msg.hex()}")
                print(f"  Message 2: {msg.hex()}")
                print(f"  Truncated MD5 (4 bytes): {truncated.hex()}")
                print(f"  Full MD5 of msg1: {md5_hex(other_msg)}")
                print(f"  Full MD5 of msg2: {md5_hex(msg)}")
                print()
                print(f"  Expected ~65536 attempts (2^16), got {attempts}")
                ratio = attempts / 65536
                print(f"  Ratio: {ratio:.2f}x the theoretical expectation")
                break
        else:
            seen[truncated] = msg

    print()
    print("This is a GENERIC attack — it works against any hash function.")
    print("The defense is a longer hash: SHA-256 has 2^128 birthday bound,")
    print("while MD5 has only 2^64 (and dedicated attacks are even faster).")
    print()


# ============================================================================
# Main
# ============================================================================

def main() -> None:
    """Run all three attacks demonstrating MD5's weaknesses."""
    print()
    print("╔══════════════════════════════════════════════════════════════════════╗")
    print("║             MD5 HASH BREAKER — Why MD5 Is Broken                   ║")
    print("╠══════════════════════════════════════════════════════════════════════╣")
    print("║  Three attacks showing MD5 must NEVER be used for security:        ║")
    print("║    1. Known collision pairs (Wang & Yu, 2004)                      ║")
    print("║    2. Length extension attack (forge MAC without secret)            ║")
    print("║    3. Birthday attack on truncated hash (birthday paradox)         ║")
    print("╚══════════════════════════════════════════════════════════════════════╝")
    print()

    attack_1_known_collision()
    attack_2_length_extension()
    attack_3_birthday()

    print("=" * 72)
    print("CONCLUSION")
    print("=" * 72)
    print()
    print("MD5 is broken in three distinct ways:")
    print("  1. COLLISION RESISTANCE: known pairs exist (and can be generated)")
    print("  2. LENGTH EXTENSION: Merkle-Damgard structure leaks internal state")
    print("  3. BIRTHDAY BOUND: only 2^64 (and dedicated attacks beat even that)")
    print()
    print("Use SHA-256 or SHA-3 for security. Use HMAC (not raw hash) for MACs.")
    print()


if __name__ == "__main__":
    main()
