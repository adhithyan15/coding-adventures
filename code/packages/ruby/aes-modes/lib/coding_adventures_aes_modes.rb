# frozen_string_literal: true

# =============================================================================
# coding_adventures_aes_modes --- AES Modes of Operation (ECB, CBC, CTR, GCM)
# =============================================================================
#
# Why Do We Need Modes?
# ---------------------
#
# AES is a *block cipher*: it encrypts exactly 16 bytes (128 bits) at a time.
# Real messages are rarely exactly 16 bytes. A "mode of operation" defines
# how to use a block cipher to encrypt messages of arbitrary length.
#
# Security hierarchy:
#
#   ECB  <  CBC  <  CTR  <  GCM
# (broken) (legacy) (good) (best)
#
# ECB and CBC are implemented here purely for educational purposes.
# In production, always use GCM (or another AEAD construction).

require_relative "coding_adventures/aes_modes/version"
require "coding_adventures_aes"

module CodingAdventures
  module AesModes
    BLOCK_SIZE = 16

    # =========================================================================
    # PKCS#7 Padding
    # =========================================================================
    #
    # Block ciphers in ECB and CBC mode require the plaintext to be an exact
    # multiple of the block size. PKCS#7 padding appends N bytes of value N,
    # where N = 16 - (len % 16). Even aligned data gets a full block of 0x10.

    # Pad data to a multiple of 16 bytes using PKCS#7.
    #
    # @param data [String] binary string to pad
    # @return [String] padded binary string (always longer than input)
    def self.pkcs7_pad(data)
      pad_len = BLOCK_SIZE - (data.bytesize % BLOCK_SIZE)
      data + ([pad_len].pack("C") * pad_len)
    end

    # Remove PKCS#7 padding and return the original data.
    #
    # @param data [String] padded binary string (must be multiple of 16)
    # @return [String] unpadded binary string
    # @raise [ArgumentError] if padding is invalid
    def self.pkcs7_unpad(data)
      if data.bytesize == 0 || (data.bytesize % BLOCK_SIZE) != 0
        raise ArgumentError, "Data length #{data.bytesize} is not a positive multiple of #{BLOCK_SIZE}"
      end

      pad_len = data.getbyte(data.bytesize - 1)

      if pad_len < 1 || pad_len > BLOCK_SIZE
        raise ArgumentError, "Invalid PKCS#7 padding"
      end

      # Constant-time padding validation: accumulate differences with OR
      # instead of returning early on the first mismatch (prevents timing attacks)
      diff = 0
      1.upto(pad_len) do |i|
        diff |= data.getbyte(data.bytesize - i) ^ pad_len
      end

      if diff != 0
        raise ArgumentError, "Invalid PKCS#7 padding"
      end

      data.byteslice(0, data.bytesize - pad_len)
    end

    # =========================================================================
    # Utility: XOR two binary strings
    # =========================================================================

    # XOR two binary strings of equal length.
    #
    # @param a [String] first binary string
    # @param b [String] second binary string
    # @return [String] XOR result
    def self.xor_bytes(a, b)
      a.bytes.zip(b.bytes).map { |x, y| x ^ y }.pack("C*")
    end

    # =========================================================================
    # ECB Mode (Electronic Codebook) --- INSECURE
    # =========================================================================
    #
    # Each 16-byte block is encrypted independently. Identical plaintext
    # blocks produce identical ciphertext blocks, revealing patterns.

    # Encrypt plaintext using AES in ECB mode (INSECURE --- educational only).
    #
    # @param plaintext [String] arbitrary-length binary data
    # @param key [String] 16, 24, or 32 bytes
    # @return [String] ciphertext (multiple of 16 bytes)
    def self.ecb_encrypt(plaintext, key)
      padded = pkcs7_pad(plaintext)
      ciphertext = "".b

      0.step(padded.bytesize - 1, BLOCK_SIZE) do |i|
        block = padded.byteslice(i, BLOCK_SIZE)
        ciphertext << CodingAdventures::Aes.aes_encrypt_block(block, key)
      end

      ciphertext
    end

    # Decrypt ciphertext that was encrypted with AES-ECB.
    #
    # @param ciphertext [String] must be non-empty multiple of 16 bytes
    # @param key [String] same key used for encryption
    # @return [String] original plaintext
    def self.ecb_decrypt(ciphertext, key)
      if ciphertext.bytesize == 0 || (ciphertext.bytesize % BLOCK_SIZE) != 0
        raise ArgumentError, "Ciphertext length #{ciphertext.bytesize} is not a positive multiple of #{BLOCK_SIZE}"
      end

      plaintext = "".b

      0.step(ciphertext.bytesize - 1, BLOCK_SIZE) do |i|
        block = ciphertext.byteslice(i, BLOCK_SIZE)
        plaintext << CodingAdventures::Aes.aes_decrypt_block(block, key)
      end

      pkcs7_unpad(plaintext)
    end

    # =========================================================================
    # CBC Mode (Cipher Block Chaining) --- Legacy
    # =========================================================================
    #
    # Each plaintext block is XOR'd with the previous ciphertext block before
    # encryption. A random IV starts the chain.

    # Encrypt plaintext using AES in CBC mode.
    #
    # @param plaintext [String] arbitrary-length binary data
    # @param key [String] 16, 24, or 32 bytes
    # @param iv [String] exactly 16 random bytes
    # @return [String] ciphertext (multiple of 16 bytes)
    def self.cbc_encrypt(plaintext, key, iv)
      raise ArgumentError, "IV must be #{BLOCK_SIZE} bytes, got #{iv.bytesize}" unless iv.bytesize == BLOCK_SIZE

      padded = pkcs7_pad(plaintext)
      ciphertext = "".b
      prev = iv

      0.step(padded.bytesize - 1, BLOCK_SIZE) do |i|
        block = padded.byteslice(i, BLOCK_SIZE)
        xored = xor_bytes(block, prev)
        encrypted = CodingAdventures::Aes.aes_encrypt_block(xored, key)
        ciphertext << encrypted
        prev = encrypted
      end

      ciphertext
    end

    # Decrypt ciphertext that was encrypted with AES-CBC.
    #
    # @param ciphertext [String] must be non-empty multiple of 16 bytes
    # @param key [String] same key used for encryption
    # @param iv [String] same IV used for encryption (16 bytes)
    # @return [String] original plaintext
    def self.cbc_decrypt(ciphertext, key, iv)
      raise ArgumentError, "IV must be #{BLOCK_SIZE} bytes, got #{iv.bytesize}" unless iv.bytesize == BLOCK_SIZE

      if ciphertext.bytesize == 0 || (ciphertext.bytesize % BLOCK_SIZE) != 0
        raise ArgumentError, "Ciphertext length #{ciphertext.bytesize} is not a positive multiple of #{BLOCK_SIZE}"
      end

      plaintext = "".b
      prev = iv

      0.step(ciphertext.bytesize - 1, BLOCK_SIZE) do |i|
        block = ciphertext.byteslice(i, BLOCK_SIZE)
        decrypted = CodingAdventures::Aes.aes_decrypt_block(block, key)
        plaintext << xor_bytes(decrypted, prev)
        prev = block
      end

      pkcs7_unpad(plaintext)
    end

    # =========================================================================
    # CTR Mode (Counter Mode) --- Recommended
    # =========================================================================
    #
    # Turns the block cipher into a stream cipher by encrypting a counter
    # and XOR-ing the keystream with the plaintext.
    #
    # Counter block format: [12-byte nonce] [4-byte big-endian counter]
    # Counter starts at 1 (GCM reserves counter 0/1 for the tag).

    # Build a 16-byte counter block: 12-byte nonce || 4-byte big-endian counter.
    #
    # @param nonce [String] 12 bytes
    # @param counter [Integer] counter value
    # @return [String] 16-byte counter block
    def self.build_counter_block(nonce, counter)
      nonce + [counter].pack("N")
    end

    # Encrypt plaintext using AES in CTR mode.
    #
    # @param plaintext [String] arbitrary-length binary data (no padding needed)
    # @param key [String] 16, 24, or 32 bytes
    # @param nonce [String] exactly 12 bytes, MUST be unique per message
    # @return [String] ciphertext (same length as plaintext)
    def self.ctr_encrypt(plaintext, key, nonce)
      raise ArgumentError, "Nonce must be 12 bytes, got #{nonce.bytesize}" unless nonce.bytesize == 12

      ciphertext = "".b
      counter = 1

      0.step([plaintext.bytesize - 1, 0].max, BLOCK_SIZE) do |i|
        break if i >= plaintext.bytesize

        counter_block = build_counter_block(nonce, counter)
        keystream = CodingAdventures::Aes.aes_encrypt_block(counter_block, key)

        chunk = plaintext.byteslice(i, BLOCK_SIZE)
        ciphertext << xor_bytes(keystream.byteslice(0, chunk.bytesize), chunk)

        counter += 1
      end

      ciphertext
    end

    # Decrypt ciphertext encrypted with AES-CTR.
    # Identical to ctr_encrypt because XOR is its own inverse.
    #
    # @param ciphertext [String] data to decrypt
    # @param key [String] same key used for encryption
    # @param nonce [String] same nonce used for encryption (12 bytes)
    # @return [String] original plaintext
    def self.ctr_decrypt(ciphertext, key, nonce)
      ctr_encrypt(ciphertext, key, nonce)
    end

    # =========================================================================
    # GCM Mode (Galois/Counter Mode) --- Recommended with Authentication
    # =========================================================================
    #
    # GCM = CTR encryption + GHASH authentication tag.
    #
    # GHASH operates in GF(2^128) with reducing polynomial:
    #   R(x) = x^128 + x^7 + x^2 + x + 1
    #
    # This is a DIFFERENT field from the GF(2^8) used inside AES itself.

    # Convert a 128-bit integer to a 16-byte binary string (big-endian).
    #
    # @param n [Integer] 128-bit integer
    # @return [String] 16-byte binary string
    def self.int_to_bytes16(n)
      result = "".b
      16.times do
        result.prepend([n & 0xFF].pack("C"))
        n >>= 8
      end
      result
    end

    # Multiply two 128-bit elements in GF(2^128) with the GCM polynomial.
    #
    # Uses the "shift-and-add" algorithm in the reflected bit convention:
    # process bits of Y from MSB to LSB, right-shift V, XOR with R on carry.
    #
    # @param x [String] 16 bytes
    # @param y [String] 16 bytes
    # @return [String] 16 bytes (product in GF(2^128))
    def self.gf128_mul(x, y)
      x_int = x.bytes.inject(0) { |acc, b| (acc << 8) | b }
      y_int = y.bytes.inject(0) { |acc, b| (acc << 8) | b }

      r = 0xE1000000000000000000000000000000

      z = 0
      v = x_int

      128.times do |i|
        z ^= v if (y_int >> (127 - i)) & 1 == 1

        carry = v & 1
        v >>= 1
        v ^= r if carry == 1
      end

      int_to_bytes16(z)
    end

    # Compute GHASH over data using hash subkey H.
    #
    # @param h [String] 16-byte hash subkey
    # @param data [String] data to hash (processed in 16-byte blocks)
    # @return [String] 16-byte GHASH digest
    def self.ghash(h, data)
      y = "\x00".b * 16

      0.step([data.bytesize - 1, 0].max, 16) do |i|
        break if i >= data.bytesize

        block = data.byteslice(i, 16) || "".b
        block = block + ("\x00" * (16 - block.bytesize)) if block.bytesize < 16
        y = gf128_mul(xor_bytes(y, block), h)
      end

      y
    end

    # Zero-pad data to a multiple of 16 bytes.
    #
    # @param data [String] binary data
    # @return [String] zero-padded data
    def self.pad_to_16(data)
      remainder = data.bytesize % 16
      return data if remainder == 0

      data + ("\x00" * (16 - remainder))
    end

    # Encrypt and authenticate using AES-GCM.
    #
    # @param plaintext [String] data to encrypt
    # @param key [String] 16, 24, or 32 bytes
    # @param iv [String] exactly 12 bytes (MUST be unique per message)
    # @param aad [String] additional authenticated data (default: empty)
    # @return [Array(String, String)] [ciphertext, 16-byte tag]
    def self.gcm_encrypt(plaintext, key, iv, aad = "".b)
      raise ArgumentError, "IV must be 12 bytes, got #{iv.bytesize}" unless iv.bytesize == 12

      # Step 1: Hash subkey H = AES(0^128, key)
      h = CodingAdventures::Aes.aes_encrypt_block("\x00".b * 16, key)

      # Step 2: Initial counter J0 = IV || 0x00000001
      j0 = iv + [1].pack("N")

      # Step 3: CTR encryption starting at counter=2
      ciphertext = "".b
      counter = 2

      0.step([plaintext.bytesize - 1, 0].max, BLOCK_SIZE) do |i|
        break if i >= plaintext.bytesize

        counter_block = build_counter_block(iv, counter)
        keystream = CodingAdventures::Aes.aes_encrypt_block(counter_block, key)
        chunk = plaintext.byteslice(i, BLOCK_SIZE)
        ciphertext << xor_bytes(keystream.byteslice(0, chunk.bytesize), chunk)
        counter += 1
      end

      # Step 4: Compute authentication tag
      len_block = [aad.bytesize * 8].pack("Q>") + [ciphertext.bytesize * 8].pack("Q>")
      ghash_input = pad_to_16(aad) + pad_to_16(ciphertext) + len_block
      s = ghash(h, ghash_input)

      enc_j0 = CodingAdventures::Aes.aes_encrypt_block(j0, key)
      tag = xor_bytes(s, enc_j0)

      [ciphertext, tag]
    end

    # Decrypt and verify using AES-GCM.
    #
    # @param ciphertext [String] data to decrypt
    # @param key [String] same key used for encryption
    # @param iv [String] same IV used for encryption (12 bytes)
    # @param aad [String] same AAD used for encryption
    # @param tag [String] 16-byte authentication tag
    # @return [String] decrypted plaintext
    # @raise [ArgumentError] if tag does not match
    def self.gcm_decrypt(ciphertext, key, iv, aad = "".b, tag = "".b)
      raise ArgumentError, "IV must be 12 bytes, got #{iv.bytesize}" unless iv.bytesize == 12
      raise ArgumentError, "Tag must be 16 bytes, got #{tag.bytesize}" unless tag.bytesize == 16

      # Recompute expected tag
      h = CodingAdventures::Aes.aes_encrypt_block("\x00".b * 16, key)
      j0 = iv + [1].pack("N")

      len_block = [aad.bytesize * 8].pack("Q>") + [ciphertext.bytesize * 8].pack("Q>")
      ghash_input = pad_to_16(aad) + pad_to_16(ciphertext) + len_block
      s = ghash(h, ghash_input)

      enc_j0 = CodingAdventures::Aes.aes_encrypt_block(j0, key)
      expected_tag = xor_bytes(s, enc_j0)

      # Constant-time tag comparison: accumulate byte differences with OR
      # instead of short-circuiting on the first mismatch (prevents timing attacks)
      diff = 0
      16.times do |i|
        diff |= expected_tag.getbyte(i) ^ tag.getbyte(i)
      end

      unless diff == 0
        raise ArgumentError, "Authentication tag mismatch --- ciphertext may have been tampered with"
      end

      # Decrypt using CTR starting at counter=2
      plaintext = "".b
      counter = 2

      0.step([ciphertext.bytesize - 1, 0].max, BLOCK_SIZE) do |i|
        break if i >= ciphertext.bytesize

        counter_block = build_counter_block(iv, counter)
        keystream = CodingAdventures::Aes.aes_encrypt_block(counter_block, key)
        chunk = ciphertext.byteslice(i, BLOCK_SIZE)
        plaintext << xor_bytes(keystream.byteslice(0, chunk.bytesize), chunk)
        counter += 1
      end

      plaintext
    end
  end
end
