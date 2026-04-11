# frozen_string_literal: true

# coding_adventures_pbkdf2 -- PBKDF2 (Password-Based Key Derivation Function 2)
# RFC 8018 (formerly RFC 2898 / PKCS#5 v2.1)
#
# == What Is PBKDF2?
#
# PBKDF2 derives a cryptographic key from a password by applying a pseudorandom
# function (PRF) — typically HMAC — +c+ times per output block. The iteration
# count +c+ is the tunable cost: every brute-force guess requires the same +c+
# PRF calls as the original derivation.
#
# Real-world deployments:
# * WPA2 Wi-Fi — PBKDF2-HMAC-SHA1, 4096 iterations
# * Django password hasher — PBKDF2-HMAC-SHA256, 720,000 iterations (2024)
# * macOS Keychain — PBKDF2-HMAC-SHA256
#
# == Algorithm (RFC 8018 § 5.2)
#
#   DK = T_1 || T_2 || ... (first dk_len bytes)
#
#   T_i = U_1 XOR U_2 XOR ... XOR U_c
#
#   U_1 = PRF(password, salt + INT_32_BE(i))
#   U_j = PRF(password, U_{j-1})   for j = 2..c
#
# INT_32_BE(i) encodes the block counter as a 4-byte big-endian integer.
# This makes each block's seed unique even when the salt repeats.
#
# == Security Notes
#
# OWASP 2023 minimum iteration counts:
# * HMAC-SHA256: 600,000 iterations
# * HMAC-SHA1:   1,300,000 iterations
#
# For new systems prefer Argon2id (memory-hard, resists GPU attacks).

require "coding_adventures_hmac"
require_relative "coding_adventures/pbkdf2/version"

module CodingAdventures
  # PBKDF2 — Password-Based Key Derivation Function 2 (RFC 8018).
  module PBKDF2
    # Generic PBKDF2 loop. Not part of the public API; use the concrete
    # variants below.
    #
    # @param prf       [Proc]    PRF(key, msg) → String (binary, h_len bytes)
    # @param h_len     [Integer] Output byte length of prf
    # @param password  [String]  Secret being stretched (binary encoding)
    # @param salt      [String]  Unique random value per credential (binary)
    # @param iterations [Integer] Number of PRF calls per block
    # @param key_length [Integer] Number of derived bytes
    # @return [String] Derived key (binary encoding)
    def self._pbkdf2(prf, h_len, password, salt, iterations, key_length)
      raise ArgumentError, "PBKDF2 password must not be empty" if password.empty?
      raise ArgumentError, "PBKDF2 iterations must be positive" unless iterations.is_a?(Integer) && iterations > 0
      raise ArgumentError, "PBKDF2 key_length must be positive" unless key_length.is_a?(Integer) && key_length > 0

      num_blocks = (key_length.to_f / h_len).ceil
      dk = "".b

      num_blocks.times do |idx|
        i = idx + 1

        # Seed = salt + INT_32_BE(i)
        # [i].pack("N") produces a 4-byte big-endian unsigned integer.
        seed = (salt + [i].pack("N")).b

        # U_1 = PRF(password, seed)
        u = prf.call(password, seed)
        t = u.b.dup

        (iterations - 1).times do
          u = prf.call(password, u)
          # XOR each byte of u into the accumulator t.
          h_len.times { |k| t.setbyte(k, t.getbyte(k) ^ u.getbyte(k)) }
        end

        dk << t
      end

      dk[0, key_length]
    end
    private_class_method :_pbkdf2

    # ──────────────────────────────────────────────────────────────────────────
    # Public API — concrete PRF variants
    # ──────────────────────────────────────────────────────────────────────────

    # PBKDF2 with HMAC-SHA1.
    #
    # hLen = 20 bytes. Used in WPA2 (4096 iterations).
    # For new systems prefer {pbkdf2_hmac_sha256}.
    #
    # @example RFC 6070 test vector
    #   PBKDF2.pbkdf2_hmac_sha1("password", "salt", 1, 20).unpack1("H*")
    #   #=> "0c60c80f961f0e71f3a9b524af6012062fe037a6"
    def self.pbkdf2_hmac_sha1(password, salt, iterations, key_length)
      prf = ->(key, msg) { CodingAdventures::Hmac.hmac_sha1(key, msg) }
      _pbkdf2(prf, 20, password.b, salt.b, iterations, key_length)
    end

    # PBKDF2 with HMAC-SHA256.
    #
    # hLen = 32 bytes. Recommended for new systems (OWASP 2023: ≥ 600,000 iterations).
    def self.pbkdf2_hmac_sha256(password, salt, iterations, key_length)
      prf = ->(key, msg) { CodingAdventures::Hmac.hmac_sha256(key, msg) }
      _pbkdf2(prf, 32, password.b, salt.b, iterations, key_length)
    end

    # PBKDF2 with HMAC-SHA512.
    #
    # hLen = 64 bytes. Suitable for high-security applications.
    def self.pbkdf2_hmac_sha512(password, salt, iterations, key_length)
      prf = ->(key, msg) { CodingAdventures::Hmac.hmac_sha512(key, msg) }
      _pbkdf2(prf, 64, password.b, salt.b, iterations, key_length)
    end

    # ──────────────────────────────────────────────────────────────────────────
    # Hex variants
    # ──────────────────────────────────────────────────────────────────────────

    # Like {pbkdf2_hmac_sha1} but returns a lowercase hex string.
    def self.pbkdf2_hmac_sha1_hex(password, salt, iterations, key_length)
      pbkdf2_hmac_sha1(password, salt, iterations, key_length).unpack1("H*")
    end

    # Like {pbkdf2_hmac_sha256} but returns a lowercase hex string.
    def self.pbkdf2_hmac_sha256_hex(password, salt, iterations, key_length)
      pbkdf2_hmac_sha256(password, salt, iterations, key_length).unpack1("H*")
    end

    # Like {pbkdf2_hmac_sha512} but returns a lowercase hex string.
    def self.pbkdf2_hmac_sha512_hex(password, salt, iterations, key_length)
      pbkdf2_hmac_sha512(password, salt, iterations, key_length).unpack1("H*")
    end
  end
end
