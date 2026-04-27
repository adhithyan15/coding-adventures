# frozen_string_literal: true

# coding_adventures_hkdf — HKDF (HMAC-based Extract-and-Expand Key Derivation
# Function, RFC 5869), implemented from scratch in Ruby.
#
# What Is Key Derivation?
# =======================
# Many cryptographic protocols start with some "input keying material" (IKM)
# that is not directly suitable as a cryptographic key. The IKM might come
# from a Diffie-Hellman exchange, a password, a random seed, or any other
# source of entropy. A Key Derivation Function (KDF) transforms this raw
# material into one or more cryptographically strong keys.
#
# HKDF is the most widely used KDF in modern cryptography. It appears in:
#
#   - TLS 1.3 (the key schedule is built entirely on HKDF)
#   - Signal Protocol (Double Ratchet key derivation)
#   - WireGuard (handshake key derivation)
#   - Noise Protocol Framework
#   - Web Crypto API (deriveBits / deriveKey)
#
# Why Two Phases?
# ===============
# HKDF splits key derivation into two distinct phases:
#
#   1. **Extract** — concentrate the entropy from the IKM into a fixed-size
#      pseudorandom key (PRK). This step "cleans up" non-uniform input.
#
#   2. **Expand** — stretch the PRK into as many output bytes as needed,
#      using an "info" string for domain separation.
#
# Visual Overview
# ===============
#
#   Input Keying Material (IKM)
#          |
#          v
#   +--------------+
#   |   Extract    |  PRK = HMAC(salt, IKM)
#   |  (compress)  |
#   +--------------+
#          |
#          v
#   Pseudorandom Key (PRK)   [exactly HashLen bytes]
#          |
#          v
#   +--------------+
#   |   Expand     |  OKM = T(1) || T(2) || ... || T(N)
#   |  (stretch)   |  where T(i) = HMAC(PRK, T(i-1) || info || i)
#   +--------------+
#          |
#          v
#   Output Keying Material (OKM)  [L bytes, up to 255 * HashLen]

require "coding_adventures_hmac"
require_relative "coding_adventures/hkdf/version"

module CodingAdventures
  module HKDF
    # ========================================================================
    # Hash Algorithm Configuration
    # ========================================================================
    #
    # Each supported hash algorithm has a fixed output length (HashLen) and
    # a corresponding HMAC function. These determine the PRK size, default
    # salt size, and maximum output length.
    #
    # ========================================================================

    # Hash output lengths in bytes.
    # SHA-256 produces 32 bytes; SHA-512 produces 64 bytes.
    HASH_LENGTHS = {
      "sha256" => 32,
      "sha512" => 64
    }.freeze

    # ========================================================================
    # HKDF-Extract
    # ========================================================================
    #
    # Extract compresses the input keying material into a fixed-size
    # pseudorandom key:
    #
    #   PRK = HMAC-Hash(salt, IKM)
    #
    # The salt is the HMAC key; IKM is the message. If salt is empty or nil,
    # we use HashLen zero bytes as the key per RFC 5869 Section 2.2.
    #
    # ========================================================================

    # Compress input keying material into a pseudorandom key.
    #
    # @param salt [String] Optional salt (binary string). Nil or empty means
    #   HashLen zero bytes.
    # @param ikm [String] Input keying material (binary string).
    # @param hash [String] Hash algorithm: "sha256" (default) or "sha512".
    # @return [String] PRK — pseudorandom key (binary string of HashLen bytes).
    def self.hkdf_extract(salt, ikm, hash = "sha256")
      hash_len = HASH_LENGTHS.fetch(hash) { raise ArgumentError, "Unsupported hash: #{hash}" }

      # RFC 5869: if salt is not provided, it is set to a string of HashLen zeros.
      effective_salt = (salt.nil? || salt.empty?) ? ("\x00" * hash_len) : salt

      hmac_fn(hash, effective_salt, ikm)
    end

    # ========================================================================
    # HKDF-Expand
    # ========================================================================
    #
    # Expand stretches the PRK into output keying material of any desired
    # length, up to 255 * HashLen bytes.
    #
    # The expansion chains HMAC calls:
    #
    #   T(0) = ""  (empty)
    #   T(1) = HMAC(PRK, T(0) || info || 0x01)
    #   T(2) = HMAC(PRK, T(1) || info || 0x02)
    #   ...
    #   T(N) = HMAC(PRK, T(N-1) || info || N)
    #
    #   OKM = first L bytes of T(1) || T(2) || ... || T(N)
    #
    # The counter is a single byte (1..255), limiting N to 255.
    #
    # ========================================================================

    # Stretch a pseudorandom key into output keying material.
    #
    # @param prk [String] Pseudorandom key (binary string, at least HashLen bytes).
    # @param info [String] Context/application info for domain separation (binary string).
    # @param length [Integer] Desired output length in bytes (1..255*HashLen).
    # @param hash [String] Hash algorithm: "sha256" (default) or "sha512".
    # @return [String] OKM — output keying material (binary string of exactly length bytes).
    # @raise [ArgumentError] If length is out of range.
    def self.hkdf_expand(prk, info, length, hash = "sha256")
      hash_len = HASH_LENGTHS.fetch(hash) { raise ArgumentError, "Unsupported hash: #{hash}" }

      if length <= 0
        raise ArgumentError, "HKDF-Expand: length must be > 0, got #{length}"
      end

      max_length = 255 * hash_len
      if length > max_length
        raise ArgumentError,
              "HKDF-Expand: length #{length} exceeds maximum #{max_length} (255 * #{hash_len}) for #{hash}"
      end

      # N = ceil(L / HashLen)
      n = (length + hash_len - 1) / hash_len

      # Build OKM by chaining HMAC blocks.
      # T(0) is the empty string.
      previous = "".b
      okm = "".b

      (1..n).each do |i|
        # Input = T(i-1) || info || counter_byte
        # The counter is a single octet with value i (1-indexed).
        input = previous + (info || "".b) + [i].pack("C")
        previous = hmac_fn(hash, prk, input)
        okm << previous
      end

      # Return exactly L bytes (truncating the last block if needed).
      okm[0, length]
    end

    # ========================================================================
    # Combined HKDF (Extract + Expand)
    # ========================================================================
    #
    # Most callers want the full pipeline: Extract then Expand. This method
    # chains both steps into a single call.
    #
    # ========================================================================

    # Full HKDF: extract-then-expand in one call.
    #
    # @param salt [String] Optional salt (binary string).
    # @param ikm [String] Input keying material (binary string).
    # @param info [String] Context info for domain separation (binary string).
    # @param length [Integer] Desired output length in bytes.
    # @param hash [String] Hash algorithm: "sha256" (default) or "sha512".
    # @return [String] OKM — derived keying material (binary string).
    def self.hkdf(salt, ikm, info, length, hash = "sha256")
      prk = hkdf_extract(salt, ikm, hash)
      hkdf_expand(prk, info, length, hash)
    end

    # ========================================================================
    # Internal: HMAC dispatch
    # ========================================================================

    # @api private
    # Dispatch to the correct HMAC function based on the hash algorithm name.
    def self.hmac_fn(hash, key, message)
      case hash
      when "sha256"
        CodingAdventures::Hmac.hmac_sha256(key, message)
      when "sha512"
        CodingAdventures::Hmac.hmac_sha512(key, message)
      else
        raise ArgumentError, "Unsupported hash: #{hash}"
      end
    end
    private_class_method :hmac_fn
  end
end
