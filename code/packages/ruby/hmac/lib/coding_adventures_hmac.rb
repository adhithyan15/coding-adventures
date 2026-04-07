# frozen_string_literal: true

# coding_adventures_hmac — HMAC (Hash-based Message Authentication Code)
# RFC 2104 / FIPS 198-1, implemented from scratch in Ruby.
#
# What Is HMAC?
# =============
# HMAC takes a secret key and a message and produces a fixed-size authentication
# tag that proves two things simultaneously:
#
#   1. Integrity   — the message has not been altered since the tag was created.
#   2. Authenticity — the sender possesses the secret key.
#
# Unlike a plain hash (which anyone can compute from a message), an HMAC tag
# cannot be forged without the key. HMAC is used everywhere authentication tags
# are needed:
#
#   - TLS 1.2 PRF (Pseudorandom Function for key derivation)
#   - JWT "HS256" and "HS512" signature algorithms
#   - WPA2 four-way handshake (PBKDF2-HMAC-SHA1 key stretching)
#   - TOTP/HOTP one-time passwords (RFC 6238 / 4226)
#   - AWS Signature Version 4 request signing
#   - Cookie signing in Rails, Django, Express.js
#
# Why Not hash(key || message)?
# ==============================
# Naively prepending the key is vulnerable to the "length extension attack"
# on Merkle-Damgård hash functions (MD5, SHA-1, SHA-256, SHA-512).
#
# Merkle-Damgård hashes maintain an internal state that is "absorbed" block
# by block. When they output a digest, they output that internal state directly.
# This means:
#
#   hash(key || msg) ≡ internal_state_after(key || msg)
#
# An attacker who knows hash(key || msg) can set that as a starting state and
# continue hashing — appending arbitrary extra bytes — without knowing `key`:
#
#   known:    hash(key || msg) = D
#   attacker: hash(key || msg || padding || extra) = D'
#             by resuming from state D (without knowing key)
#
# This breaks authentication: the attacker extends a valid tag with extra data.
#
# HMAC fixes this by wrapping the message in two layers of hashing under
# different derived keys:
#
#   HMAC(K, M) = H((K' XOR opad) || H((K' XOR ipad) || M))
#
# The outer hash takes the inner result as a fresh message — the attacker
# cannot "resume" it without knowing K' XOR opad, which requires knowing K.
#
# The ipad and opad Constants
# ============================
# RFC 2104 defines:
#   ipad = 0x36 = 0011_0110  (inner pad)
#   opad = 0x5C = 0101_1100  (outer pad)
#
# These two values differ in exactly 4 of 8 bits — the maximum Hamming
# distance possible for single-byte values where both are XOR'd with the
# same key byte. This ensures inner_key and outer_key are maximally different
# despite sharing the same source key K', making simultaneous attacks harder.
#
# The Algorithm (RFC 2104 §2)
# ============================
#   1. Normalize K to exactly block_size bytes:
#        len(K) > block_size → K' = H(K), then zero-pad to block_size
#        len(K) ≤ block_size → zero-pad to block_size
#   2. inner_key = K' XOR (0x36 × block_size)
#   3. outer_key = K' XOR (0x5C × block_size)
#   4. inner     = H(inner_key + message)          (+ = concatenate)
#   5. return      H(outer_key + inner)
#
# Block Sizes (bytes)
# ====================
#   MD5     → block = 64,  digest = 16
#   SHA-1   → block = 64,  digest = 20
#   SHA-256 → block = 64,  digest = 32
#   SHA-512 → block = 128, digest = 64   (SHA-512 uses 64-bit words, larger block)
#
# RFC 4231 Test Vector TC1 (HMAC-SHA256)
# ========================================
#   key = "\x0b" * 20
#   msg = "Hi There"
#   tag = "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"

require "coding_adventures_md5"
require "coding_adventures_sha1"
require "coding_adventures_sha256"
require "coding_adventures_sha512"
require_relative "coding_adventures/hmac/version"

module CodingAdventures
  module Hmac
    # ipad and opad constants (RFC 2104 §2)
    #
    # ipad = 0x36 = 0011_0110  (XOR'd with K' to form inner_key)
    # opad = 0x5C = 0101_1100  (XOR'd with K' to form outer_key)
    #
    # They differ in 4 bits — maximum Hamming distance for single-byte values
    # XOR'd with the same source — ensuring inner_key ≠ outer_key maximally.
    IPAD = 0x36
    OPAD = 0x5C

    # Generic HMAC over any hash function.
    #
    # @param hash_fn    [Proc] one-shot hash: accepts String (binary), returns String (binary)
    # @param block_size [Integer] internal block size in bytes (64 for MD5/SHA-1/SHA-256, 128 for SHA-512)
    # @param key        [String] secret key, binary encoding, any length
    # @param message    [String] data to authenticate, binary encoding, any length
    # @return [String] authentication tag as binary String
    #
    # @example Using SHA-256
    #   tag = CodingAdventures::Hmac.hmac(
    #     ->(d) { CodingAdventures::Sha256.sha256(d) },
    #     64,
    #     "\x0b" * 20,
    #     "Hi There"
    #   )
    def self.hmac(hash_fn, block_size, key, message)
      # Step 1 — normalize key to exactly block_size bytes
      key_prime = normalize_key(hash_fn, block_size, key)

      # Step 2 — derive inner and outer padded keys by XOR-ing with ipad/opad
      inner_key = xor_fill(key_prime, IPAD)
      outer_key = xor_fill(key_prime, OPAD)

      # Step 3 — nested hashes: inner = H(inner_key || message)
      # Force binary (ASCII-8BIT) encoding on all parts before concatenating.
      # Ruby distinguishes UTF-8 and binary strings and raises
      # Encoding::CompatibilityError when mixing them with `+`. Keys and
      # messages from callers may carry UTF-8 encoding even though they contain
      # arbitrary bytes, so we coerce everything to binary here.
      inner = hash_fn.call(inner_key.b + message.b)

      # Step 4 — outer = H(outer_key || inner)
      hash_fn.call(outer_key.b + inner.b)
    end

    # ─── Named variants ──────────────────────────────────────────────────────

    # HMAC-MD5: 16-byte authentication tag (RFC 2202).
    #
    # HMAC-MD5 remains secure as a MAC even though MD5 is broken for collision
    # resistance — MAC security and collision resistance are different properties.
    # It still appears in legacy TLS cipher suites and some older protocols.
    #
    # @param key [String] secret key (binary)
    # @param message [String] data to authenticate (binary)
    # @return [String] 16-byte binary authentication tag
    def self.hmac_md5(key, message)
      hmac(->(d) { CodingAdventures::Md5.md5(d) }, 64, key, message)
    end

    # HMAC-SHA1: 20-byte authentication tag (RFC 2202).
    #
    # Used in WPA2 (PBKDF2-HMAC-SHA1), older TLS/SSH handshakes, and TOTP/HOTP.
    # SHA-1 is collision-broken (2017 SHAttered attack) but HMAC-SHA1 remains
    # secure as a MAC.
    #
    # @param key [String] secret key (binary)
    # @param message [String] data to authenticate (binary)
    # @return [String] 20-byte binary authentication tag
    def self.hmac_sha1(key, message)
      hmac(->(d) { CodingAdventures::Sha1.sha1(d) }, 64, key, message)
    end

    # HMAC-SHA256: 32-byte authentication tag (RFC 4231).
    #
    # The modern default for TLS 1.3, JWT HS256, AWS Signature V4,
    # and PBKDF2-HMAC-SHA256.
    #
    # @param key [String] secret key (binary)
    # @param message [String] data to authenticate (binary)
    # @return [String] 32-byte binary authentication tag
    def self.hmac_sha256(key, message)
      hmac(->(d) { CodingAdventures::Sha256.sha256(d) }, 64, key, message)
    end

    # HMAC-SHA512: 64-byte authentication tag (RFC 4231).
    #
    # Used in JWT HS512 and high-security configurations.
    # SHA-512 uses 128-byte blocks (vs 64 for SHA-256), so ipad/opad are doubled.
    #
    # @param key [String] secret key (binary)
    # @param message [String] data to authenticate (binary)
    # @return [String] 64-byte binary authentication tag
    def self.hmac_sha512(key, message)
      hmac(->(d) { CodingAdventures::Sha512.sha512(d) }, 128, key, message)
    end

    # ─── Hex-string variants ─────────────────────────────────────────────────

    # HMAC-MD5 as a 32-character lowercase hex string.
    def self.hmac_md5_hex(key, message)
      hmac_md5(key, message).unpack1("H*")
    end

    # HMAC-SHA1 as a 40-character lowercase hex string.
    def self.hmac_sha1_hex(key, message)
      hmac_sha1(key, message).unpack1("H*")
    end

    # HMAC-SHA256 as a 64-character lowercase hex string.
    def self.hmac_sha256_hex(key, message)
      hmac_sha256(key, message).unpack1("H*")
    end

    # HMAC-SHA512 as a 128-character lowercase hex string.
    def self.hmac_sha512_hex(key, message)
      hmac_sha512(key, message).unpack1("H*")
    end

    # ─── Private helpers ─────────────────────────────────────────────────────

    # Normalize key to exactly block_size bytes.
    # Long keys are hashed with hash_fn first. All keys are zero-padded.
    #
    # @param hash_fn [Proc] hash function for long-key compression
    # @param block_size [Integer] target size in bytes
    # @param key [String] raw key bytes
    # @return [String] exactly block_size bytes (binary encoding)
    def self.normalize_key(hash_fn, block_size, key)
      effective = key.bytesize > block_size ? hash_fn.call(key) : key
      # Zero-pad to block_size on the right
      effective.b.ljust(block_size, "\x00".b)
    end

    # XOR every byte in data with constant fill byte.
    # Returns a new binary String of the same length.
    def self.xor_fill(data, fill)
      data.bytes.map { |b| b ^ fill }.pack("C*")
    end

    private_class_method :normalize_key, :xor_fill
  end
end
