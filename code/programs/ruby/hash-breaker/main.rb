#!/usr/bin/env ruby
# frozen_string_literal: true

# hash-breaker — Demonstrating why MD5 is cryptographically broken.
#
# This program runs three attacks against MD5 to show, in concrete terms, why
# you must never use MD5 for security:
#
#   1. Known Collision Pairs — two different byte sequences with the same MD5
#   2. Length Extension Attack — forge a valid hash without knowing the secret
#   3. Birthday Attack — find a collision on a truncated hash via birthday paradox
#
# Each attack prints educational output explaining the cryptographic concept.

require "coding_adventures_md5"

# ============================================================================
# ATTACK 1: Known MD5 Collision Pairs (Wang & Yu, 2004)
# ============================================================================
#
# In 2004, Xiaoyun Wang and Hongbo Yu published the first practical collision
# attack on MD5. They found two 128-byte messages that produce the SAME MD5
# hash despite differing in specific bit positions.
#
# A collision means: md5(A) == md5(B) but A != B

COLLISION_A = [
  "d131dd02c5e6eec4693d9a0698aff95c",
  "2fcab58712467eab4004583eb8fb7f89",
  "55ad340609f4b30283e488832571415a",
  "085125e8f7cdc99fd91dbdf280373c5b",
  "d8823e3156348f5bae6dacd436c919c6",
  "dd53e2b487da03fd02396306d248cda0",
  "e99f33420f577ee8ce54b67080a80d1e",
  "c69821bcb6a8839396f9652b6ff72a70"
].join.scan(/../).map { |h| h.to_i(16) }.pack("C*")

COLLISION_B = [
  "d131dd02c5e6eec4693d9a0698aff95c",
  "2fcab50712467eab4004583eb8fb7f89",
  "55ad340609f4b30283e4888325f1415a",
  "085125e8f7cdc99fd91dbd7280373c5b",
  "d8823e3156348f5bae6dacd436c919c6",
  "dd53e23487da03fd02396306d248cda0",
  "e99f33420f577ee8ce54b67080280d1e",
  "c69821bcb6a8839396f965ab6ff72a70"
].join.scan(/../).map { |h| h.to_i(16) }.pack("C*")

def hex_dump(data)
  data.bytes.each_slice(16).map { |row|
    "  " + row.map { |b| format("%02x", b) }.join
  }.join("\n")
end

def attack_1_known_collision
  puts "=" * 72
  puts "ATTACK 1: Known MD5 Collision Pair (Wang & Yu, 2004)"
  puts "=" * 72
  puts
  puts "Two different 128-byte messages that produce the SAME MD5 hash."
  puts "This was the breakthrough that proved MD5 is broken for security."
  puts

  puts "Block A (hex):"
  puts hex_dump(COLLISION_A)
  puts

  puts "Block B (hex):"
  puts hex_dump(COLLISION_B)
  puts

  # Show byte differences
  diffs = []
  COLLISION_A.bytes.each_with_index do |byte_a, i|
    diffs << i if byte_a != COLLISION_B.bytes[i]
  end
  puts "Blocks differ at #{diffs.length} byte positions: #{diffs}"
  diffs.each do |pos|
    puts format("  Byte %d: A=0x%02x  B=0x%02x", pos, COLLISION_A.bytes[pos], COLLISION_B.bytes[pos])
  end
  puts

  hash_a = CodingAdventures::Md5.md5_hex(COLLISION_A)
  hash_b = CodingAdventures::Md5.md5_hex(COLLISION_B)
  puts "MD5(A) = #{hash_a}"
  puts "MD5(B) = #{hash_b}"
  puts "Match?   #{hash_a == hash_b ? "YES — COLLISION!" : "No (unexpected)"}"
  puts
  puts "Lesson: MD5 collisions are REAL. Never use MD5 for integrity or auth."
  puts
end

# ============================================================================
# ATTACK 2: Length Extension Attack
# ============================================================================
#
# MD5 (and all Merkle-Damgard hashes) are vulnerable to length extension.
# Given md5(secret || message) and len(secret || message), an attacker can
# compute md5(secret || message || padding || evil_data) WITHOUT knowing
# the secret. This breaks naive MAC = md5(secret || message).

# MD5 per-round sine-derived constants.
T_TABLE = (0...64).map { |i| (Math.sin(i + 1).abs * (2**32)).floor & 0xFFFFFFFF }

# MD5 per-round left-rotation amounts.
SHIFTS = [
  7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22,
  5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20,
  4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23,
  6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21
].freeze

MASK32 = 0xFFFFFFFF

def rotl32(x, n)
  ((x << n) | (x >> (32 - n))) & MASK32
end

# Inline MD5 compression for the length extension attack.
# We need access to the internal compress function, so we reimplement it here.
def md5_compress(state, block)
  m = block.unpack("V16")
  a, b, c, d = state
  a0, b0, c0, d0 = a, b, c, d

  64.times do |i|
    case i
    when 0..15
      f = (b & c) | (~b & d)
      g = i
    when 16..31
      f = (d & b) | (~d & c)
      g = (5 * i + 1) % 16
    when 32..47
      f = b ^ c ^ d
      g = (3 * i + 5) % 16
    else
      f = c ^ (b | ~d)
      g = (7 * i) % 16
    end
    f &= MASK32
    temp = d
    d = c
    c = b
    b = (b + rotl32((a + f + T_TABLE[i] + m[g]) & MASK32, SHIFTS[i])) & MASK32
    a = temp
  end

  [(a0 + a) & MASK32, (b0 + b) & MASK32, (c0 + c) & MASK32, (d0 + d) & MASK32]
end

def md5_padding(message_len)
  remainder = message_len % 64
  pad_len = (55 - remainder) % 64
  padding = "\x80".b + ("\x00".b * pad_len)
  bit_len = message_len * 8
  padding + [bit_len & MASK32, (bit_len >> 32) & MASK32].pack("V2")
end

def attack_2_length_extension
  puts "=" * 72
  puts "ATTACK 2: Length Extension Attack"
  puts "=" * 72
  puts
  puts "Given md5(secret + message) and len(secret + message), we can forge"
  puts "md5(secret + message + padding + evil_data) WITHOUT knowing the secret!"
  puts

  secret = "supersecretkey!!".b
  message = "amount=100&to=alice".b
  original_data = secret + message
  original_hash = CodingAdventures::Md5.md5(original_data)
  original_hex = original_hash.unpack1("H*")

  puts "Secret (unknown to attacker): #{secret.inspect}"
  puts "Message:                      #{message.inspect}"
  puts "MAC = md5(secret || message): #{original_hex}"
  puts "Length of (secret || message): #{original_data.length} bytes"
  puts

  evil_data = "&amount=1000000&to=mallory".b
  puts "Evil data to append: #{evil_data.inspect}"
  puts

  # Step 1: Extract state from hash (four LE 32-bit words)
  a, b, c, d = original_hash.unpack("V4")
  puts "Step 1: Extract MD5 internal state from the hash"
  puts format("  A = 0x%08x, B = 0x%08x, C = 0x%08x, D = 0x%08x", a, b, c, d)
  puts

  # Step 2: Compute padding
  padding = md5_padding(original_data.length)
  puts "Step 2: Compute MD5 padding for the original message"
  puts "  Padding (#{padding.length} bytes): #{padding.unpack1("H*")}"
  puts

  processed_len = original_data.length + padding.length
  puts "Step 3: Total bytes processed so far: #{processed_len}"
  puts

  # Step 4: Forge
  forged_input = evil_data + md5_padding(processed_len + evil_data.length)
  state = [a, b, c, d]
  (0...forged_input.length).step(64) do |i|
    block = forged_input[i, 64]
    state = md5_compress(state, block) if block.length == 64
  end
  forged_hex = state.pack("V4").unpack1("H*")

  puts "Step 4: Initialize hasher with extracted state, feed evil_data"
  puts "  Forged hash: #{forged_hex}"
  puts

  # Step 5: Verify
  actual_full = original_data + padding + evil_data
  actual_hex = CodingAdventures::Md5.md5_hex(actual_full)

  puts "Step 5: Verify — compute actual md5(secret || message || padding || evil_data)"
  puts "  Actual hash: #{actual_hex}"
  puts "  Match?       #{forged_hex == actual_hex ? "YES — FORGED!" : "No (bug)"}"
  puts
  puts "The attacker forged a valid MAC without knowing the secret!"
  puts
  puts "Why HMAC fixes this:"
  puts "  HMAC = md5(key XOR opad || md5(key XOR ipad || message))"
  puts "  The outer hash prevents length extension because the attacker"
  puts "  cannot extend past the outer md5() boundary."
  puts
end

# ============================================================================
# ATTACK 3: Birthday Attack (Truncated Hash)
# ============================================================================
#
# The birthday paradox: with N possible values, expect a collision after
# roughly sqrt(N) random samples. For 32-bit truncated hash: 2^16 = 65536.

def attack_3_birthday
  puts "=" * 72
  puts "ATTACK 3: Birthday Attack on Truncated MD5 (32-bit)"
  puts "=" * 72
  puts
  puts "The birthday paradox: with N possible hash values, expect a collision"
  puts "after ~sqrt(N) random inputs. For 32-bit hash: sqrt(2^32) = 2^16 = 65536."
  puts

  srand(42)
  seen = {}
  attempts = 0

  loop do
    attempts += 1
    msg = Array.new(8) { rand(256) }.pack("C*")
    full_hash = CodingAdventures::Md5.md5(msg)
    truncated = full_hash[0, 4]

    if seen.key?(truncated)
      other_msg = seen[truncated]
      if other_msg != msg
        puts "COLLISION FOUND after #{attempts} attempts!"
        puts
        puts "  Message 1: #{other_msg.unpack1("H*")}"
        puts "  Message 2: #{msg.unpack1("H*")}"
        puts "  Truncated MD5 (4 bytes): #{truncated.unpack1("H*")}"
        puts "  Full MD5 of msg1: #{CodingAdventures::Md5.md5_hex(other_msg)}"
        puts "  Full MD5 of msg2: #{CodingAdventures::Md5.md5_hex(msg)}"
        puts
        puts "  Expected ~65536 attempts (2^16), got #{attempts}"
        puts format("  Ratio: %.2fx the theoretical expectation", attempts.to_f / 65536)
        break
      end
    else
      seen[truncated] = msg
    end
  end

  puts
  puts "This is a GENERIC attack — it works against any hash function."
  puts "The defense is a longer hash: SHA-256 has 2^128 birthday bound,"
  puts "while MD5 has only 2^64 (and dedicated attacks are even faster)."
  puts
end

# ============================================================================
# Main
# ============================================================================

def main
  puts
  puts "======================================================================"
  puts "           MD5 HASH BREAKER — Why MD5 Is Broken"
  puts "======================================================================"
  puts "  Three attacks showing MD5 must NEVER be used for security:"
  puts "    1. Known collision pairs (Wang & Yu, 2004)"
  puts "    2. Length extension attack (forge MAC without secret)"
  puts "    3. Birthday attack on truncated hash (birthday paradox)"
  puts "======================================================================"
  puts

  attack_1_known_collision
  attack_2_length_extension
  attack_3_birthday

  puts "=" * 72
  puts "CONCLUSION"
  puts "=" * 72
  puts
  puts "MD5 is broken in three distinct ways:"
  puts "  1. COLLISION RESISTANCE: known pairs exist (and can be generated)"
  puts "  2. LENGTH EXTENSION: Merkle-Damgard structure leaks internal state"
  puts "  3. BIRTHDAY BOUND: only 2^64 (and dedicated attacks beat even that)"
  puts
  puts "Use SHA-256 or SHA-3 for security. Use HMAC (not raw hash) for MACs."
  puts
end

main
