# frozen_string_literal: true

# ChaCha20-Poly1305: Authenticated Encryption with Associated Data (RFC 8439)
# ============================================================================
#
# This module implements the ChaCha20-Poly1305 AEAD cipher suite from scratch,
# using only basic arithmetic operations. It combines two primitives:
#
# 1. *ChaCha20* -- a stream cipher that generates a pseudorandom keystream
#    using only Add, Rotate, and XOR (ARX) operations on 32-bit words.
#
# 2. *Poly1305* -- a one-time message authentication code (MAC) that produces
#    a 16-byte tag using polynomial evaluation modulo a prime.
#
# Together, they provide *authenticated encryption*: the ciphertext is both
# confidential (only someone with the key can read it) and authentic (any
# tampering is detected).
#
# == Why ChaCha20 instead of AES?
#
# AES relies on lookup tables (S-boxes) and Galois field arithmetic that are
# complex and vulnerable to cache-timing side-channel attacks when implemented
# in software. ChaCha20 uses only additions, rotations, and XORs -- operations
# that run in constant time on all CPUs, making it naturally resistant to
# timing attacks without any special effort.
#
# Reference: RFC 8439 (https://www.rfc-editor.org/rfc/rfc8439)

require_relative "coding_adventures/chacha20_poly1305/version"

module CodingAdventures
  module Chacha20Poly1305
    # All 32-bit arithmetic must be masked to prevent Ruby's arbitrary-precision
    # integers from growing beyond 32 bits.
    MASK32 = 0xFFFFFFFF

    # The ChaCha20 state matrix begins with four constant words that spell out
    # "expand 32-byte k" in ASCII. These constants ensure that even if an
    # attacker controls the key, nonce, and counter, they cannot force the
    # initial state into a degenerate configuration.
    #
    #   "expa" = 0x61707865
    #   "nd 3" = 0x3320646e
    #   "2-by" = 0x79622d32
    #   "te k" = 0x6b206574
    CHACHA20_CONSTANTS = [0x61707865, 0x3320646e, 0x79622d32, 0x6b206574].freeze

    # The prime modulus for Poly1305: p = 2^130 - 5.
    # This is close to a power of 2, making modular arithmetic efficient.
    # Ruby has native big integers, so this works naturally.
    POLY1305_PRIME = (1 << 130) - 5

    class << self
      # -----------------------------------------------------------------------
      # ChaCha20 Stream Cipher
      # -----------------------------------------------------------------------

      # Encrypt (or decrypt) data using the ChaCha20 stream cipher.
      #
      # ChaCha20 is a *stream cipher*: it generates a pseudorandom keystream
      # and XORs it with the plaintext to produce ciphertext. Because XOR is
      # its own inverse, the same function encrypts and decrypts:
      #
      #   ciphertext = plaintext XOR keystream
      #   plaintext  = ciphertext XOR keystream
      #
      # @param plaintext [String] Data to encrypt (or ciphertext to decrypt).
      #   Must be a binary string (ASCII-8BIT encoding).
      # @param key [String] 256-bit (32-byte) secret key.
      # @param nonce [String] 96-bit (12-byte) nonce. Must be unique per key.
      # @param counter [Integer] Starting block counter (default 0).
      # @return [String] The XOR of the input with the ChaCha20 keystream.
      def chacha20_encrypt(plaintext, key, nonce, counter = 0)
        raise ArgumentError, "Key must be 32 bytes, got #{key.bytesize}" unless key.bytesize == 32
        raise ArgumentError, "Nonce must be 12 bytes, got #{nonce.bytesize}" unless nonce.bytesize == 12

        pt_bytes = plaintext.bytes
        result = []
        offset = 0

        while offset < pt_bytes.length
          # Generate one 64-byte keystream block
          keystream = chacha20_block(key, counter, nonce)

          # XOR plaintext with keystream (last block may be partial)
          chunk_end = [offset + 64, pt_bytes.length].min
          (offset...chunk_end).each do |i|
            result << (pt_bytes[i] ^ keystream[i - offset])
          end

          offset += 64
          counter += 1
        end

        result.pack("C*")
      end

      # Compute a Poly1305 one-time MAC tag.
      #
      # Poly1305 evaluates a polynomial over a prime field to produce a 16-byte
      # authentication tag. It's blazingly fast and provably secure -- but only
      # when each key is used *exactly once*. Reusing a Poly1305 key allows an
      # attacker to forge tags.
      #
      # == How it works
      #
      # 1. Split the 32-byte key into two 16-byte halves: r and s.
      # 2. "Clamp" r by clearing certain bits (ensures algebraic structure).
      # 3. Process message in 16-byte chunks:
      #    a. Interpret chunk as little-endian integer with 0x01 appended
      #    b. acc = ((acc + chunk) * r) mod (2^130 - 5)
      # 4. tag = (acc + s) mod 2^128
      #
      # @param message [String] The message to authenticate.
      # @param key [String] 32-byte one-time key.
      # @return [String] 16-byte authentication tag.
      def poly1305_mac(message, key)
        raise ArgumentError, "Poly1305 key must be 32 bytes, got #{key.bytesize}" unless key.bytesize == 32

        # Split key into r (first 16 bytes) and s (last 16 bytes)
        r_bytes = key.bytes[0, 16].dup
        s_bytes = key.bytes[16, 16]

        # Clamp r: clear specific bits for the security proof to hold.
        #   bytes 3, 7, 11, 15: clear top 4 bits (& 0x0f)
        #   bytes 4, 8, 12: clear bottom 2 bits (& 0xfc)
        r_bytes[3] &= 0x0f
        r_bytes[7] &= 0x0f
        r_bytes[11] &= 0x0f
        r_bytes[15] &= 0x0f
        r_bytes[4] &= 0xfc
        r_bytes[8] &= 0xfc
        r_bytes[12] &= 0xfc

        # Convert r and s to integers (little-endian).
        # Ruby has native big integers, so this is straightforward.
        r = le_bytes_to_int(r_bytes)
        s = le_bytes_to_int(s_bytes)

        # Process message in 16-byte blocks
        accumulator = 0
        msg_bytes = message.bytes
        i = 0

        while i < msg_bytes.length
          chunk = msg_bytes[i, 16]

          # Convert chunk to a little-endian integer, then set bit 8*len(chunk).
          # The sentinel bit distinguishes trailing zeros from padding.
          n = le_bytes_to_int(chunk)
          n |= (1 << (8 * chunk.length))

          # acc = ((acc + chunk) * r) mod p
          accumulator = ((accumulator + n) * r) % POLY1305_PRIME

          i += 16
        end

        # tag = (acc + s) mod 2^128
        tag_int = (accumulator + s) & ((1 << 128) - 1)

        # Convert to 16 little-endian bytes
        int_to_le_bytes(tag_int, 16)
      end

      # Encrypt and authenticate data using ChaCha20-Poly1305 AEAD.
      #
      # The construction (RFC 8439 Section 2.8):
      # 1. Generate Poly1305 key: first 32 bytes of ChaCha20(key, nonce, counter=0)
      # 2. Encrypt plaintext with ChaCha20(key, nonce, counter=1)
      # 3. MAC input: AAD || pad16(AAD) || CT || pad16(CT) || le64(len(AAD)) || le64(len(CT))
      # 4. tag = Poly1305(polyKey, macInput)
      #
      # @param plaintext [String] Data to encrypt.
      # @param key [String] 256-bit (32-byte) secret key.
      # @param nonce [String] 96-bit (12-byte) nonce.
      # @param aad [String] Additional authenticated data (not encrypted).
      # @return [Array(String, String)] [ciphertext, 16-byte tag].
      def aead_encrypt(plaintext, key, nonce, aad = "".b)
        raise ArgumentError, "Key must be 32 bytes, got #{key.bytesize}" unless key.bytesize == 32
        raise ArgumentError, "Nonce must be 12 bytes, got #{nonce.bytesize}" unless nonce.bytesize == 12

        # Step 1: Generate one-time Poly1305 key from ChaCha20 block 0
        poly_key_block = chacha20_block_bytes(key, 0, nonce)
        poly_key = poly_key_block[0, 32]

        # Step 2: Encrypt plaintext starting at counter=1
        ciphertext = chacha20_encrypt(plaintext, key, nonce, 1)

        # Step 3: Construct MAC input and compute tag
        mac_data = build_mac_data(aad, ciphertext)
        tag = poly1305_mac(mac_data, poly_key)

        [ciphertext, tag]
      end

      # Decrypt and verify data using ChaCha20-Poly1305 AEAD.
      #
      # If the tag doesn't match, raises an error without returning any
      # decrypted data. This prevents chosen-ciphertext attacks.
      #
      # @param ciphertext [String] Encrypted data.
      # @param key [String] 256-bit (32-byte) secret key.
      # @param nonce [String] 96-bit (12-byte) nonce.
      # @param aad [String] Additional authenticated data.
      # @param tag [String] 16-byte authentication tag.
      # @return [String] Decrypted plaintext.
      # @raise [RuntimeError] If authentication fails.
      def aead_decrypt(ciphertext, key, nonce, aad, tag)
        raise ArgumentError, "Key must be 32 bytes, got #{key.bytesize}" unless key.bytesize == 32
        raise ArgumentError, "Nonce must be 12 bytes, got #{nonce.bytesize}" unless nonce.bytesize == 12
        raise ArgumentError, "Tag must be 16 bytes, got #{tag.bytesize}" unless tag.bytesize == 16

        # Step 1: Generate one-time Poly1305 key
        poly_key_block = chacha20_block_bytes(key, 0, nonce)
        poly_key = poly_key_block[0, 32]

        # Step 2: Recompute expected tag
        mac_data = build_mac_data(aad, ciphertext)
        expected_tag = poly1305_mac(mac_data, poly_key)

        # Step 3: Constant-time tag comparison
        raise "Authentication failed: tag mismatch" unless constant_time_compare(expected_tag, tag)

        # Step 4: Decrypt
        chacha20_encrypt(ciphertext, key, nonce, 1)
      end

      private

      # -----------------------------------------------------------------------
      # 32-bit left rotation
      # -----------------------------------------------------------------------

      # Rotate a 32-bit integer left by +shift+ bits.
      #
      # In hardware, a rotation is a single instruction. In Ruby, we emulate it
      # by shifting left, shifting right the complement, and OR-ing together.
      # The mask ensures we stay within 32 bits (Ruby integers are arbitrary
      # precision, so they'd keep growing without it).
      def rotl32(value, shift)
        ((value << shift) | (value >> (32 - shift))) & MASK32
      end

      # -----------------------------------------------------------------------
      # ChaCha20 Quarter Round
      # -----------------------------------------------------------------------

      # Apply the ChaCha20 quarter round to four words in the state array.
      #
      # The quarter round is the core mixing function of ChaCha20. It takes
      # four 32-bit words and mixes them using ARX operations:
      #
      #   a += b;  d ^= a;  d <<<= 16
      #   c += d;  b ^= c;  b <<<= 12
      #   a += b;  d ^= a;  d <<<= 8
      #   c += d;  b ^= c;  b <<<= 7
      #
      # The rotation amounts (16, 12, 8, 7) were chosen by Bernstein to
      # maximize diffusion.
      def quarter_round(state, a, b, c, d)
        # Step 1
        state[a] = (state[a] + state[b]) & MASK32
        state[d] ^= state[a]
        state[d] = rotl32(state[d], 16)

        # Step 2
        state[c] = (state[c] + state[d]) & MASK32
        state[b] ^= state[c]
        state[b] = rotl32(state[b], 12)

        # Step 3
        state[a] = (state[a] + state[b]) & MASK32
        state[d] ^= state[a]
        state[d] = rotl32(state[d], 8)

        # Step 4
        state[c] = (state[c] + state[d]) & MASK32
        state[b] ^= state[c]
        state[b] = rotl32(state[b], 7)
      end

      # -----------------------------------------------------------------------
      # ChaCha20 Block Function
      # -----------------------------------------------------------------------

      # Generate one 64-byte keystream block as an array of bytes.
      #
      # The state is a 4x4 matrix of 32-bit words:
      #
      #   +----------+----------+----------+----------+
      #   | const[0] | const[1] | const[2] | const[3] |  <- "expand 32-byte k"
      #   +----------+----------+----------+----------+
      #   |  key[0]  |  key[1]  |  key[2]  |  key[3]  |  <- first half of key
      #   +----------+----------+----------+----------+
      #   |  key[4]  |  key[5]  |  key[6]  |  key[7]  |  <- second half of key
      #   +----------+----------+----------+----------+
      #   | counter  | nonce[0] | nonce[1] | nonce[2] |  <- counter + nonce
      #   +----------+----------+----------+----------+
      #
      # After 20 rounds, the original state is added back to make the
      # function one-way (without it, the mixing would be invertible).
      def chacha20_block(key, counter, nonce)
        # Unpack key as 8 little-endian 32-bit words
        key_words = key.unpack("V8")
        # Unpack nonce as 3 little-endian 32-bit words
        nonce_words = nonce.unpack("V3")

        state = [
          *CHACHA20_CONSTANTS,
          *key_words,
          counter & MASK32, *nonce_words
        ]

        # Save original state for the final addition
        initial_state = state.dup

        # 20 rounds = 10 double-rounds
        10.times do
          # Column rounds
          quarter_round(state, 0, 4, 8, 12)
          quarter_round(state, 1, 5, 9, 13)
          quarter_round(state, 2, 6, 10, 14)
          quarter_round(state, 3, 7, 11, 15)
          # Diagonal rounds
          quarter_round(state, 0, 5, 10, 15)
          quarter_round(state, 1, 6, 11, 12)
          quarter_round(state, 2, 7, 8, 13)
          quarter_round(state, 3, 4, 9, 14)
        end

        # Add original state back
        16.times { |i| state[i] = (state[i] + initial_state[i]) & MASK32 }

        # Return as array of bytes (for XOR in chacha20_encrypt)
        state.flat_map { |word| [word & 0xff, (word >> 8) & 0xff, (word >> 16) & 0xff, (word >> 24) & 0xff] }
      end

      # Same as chacha20_block but returns a binary string instead of byte array.
      # Used for AEAD key generation where we need string slicing.
      def chacha20_block_bytes(key, counter, nonce)
        chacha20_block(key, counter, nonce).pack("C*")
      end

      # -----------------------------------------------------------------------
      # Integer/byte conversion helpers
      # -----------------------------------------------------------------------

      # Convert an array of bytes (little-endian) to an integer.
      # Ruby's native big integers handle the 130+ bit arithmetic needed
      # for Poly1305 naturally.
      def le_bytes_to_int(byte_array)
        result = 0
        byte_array.each_with_index do |b, i|
          result |= (b << (8 * i))
        end
        result
      end

      # Convert an integer to a binary string of little-endian bytes.
      def int_to_le_bytes(value, length)
        bytes = Array.new(length) { |i| (value >> (8 * i)) & 0xff }
        bytes.pack("C*")
      end

      # -----------------------------------------------------------------------
      # Pad16 helper
      # -----------------------------------------------------------------------

      # Return zero-padding bytes to make the data length a multiple of 16.
      def pad16(data)
        remainder = data.bytesize % 16
        return "".b if remainder == 0
        ("\x00" * (16 - remainder)).b
      end

      # -----------------------------------------------------------------------
      # MAC data construction
      # -----------------------------------------------------------------------

      # Build the Poly1305 input for AEAD:
      #   AAD || pad16(AAD) || ciphertext || pad16(ciphertext) ||
      #   le64(len(AAD)) || le64(len(ciphertext))
      def build_mac_data(aad, ciphertext)
        data = "".b
        data << aad
        data << pad16(aad)
        data << ciphertext
        data << pad16(ciphertext)
        data << [aad.bytesize].pack("Q<")
        data << [ciphertext.bytesize].pack("Q<")
        data
      end

      # -----------------------------------------------------------------------
      # Constant-time comparison
      # -----------------------------------------------------------------------

      # Compare two byte strings in constant time to avoid timing side channels.
      def constant_time_compare(a, b)
        return false unless a.bytesize == b.bytesize

        result = 0
        a.bytes.zip(b.bytes) { |x, y| result |= x ^ y }
        result == 0
      end
    end
  end
end
