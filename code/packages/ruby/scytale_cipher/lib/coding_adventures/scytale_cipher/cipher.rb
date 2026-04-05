# frozen_string_literal: true

# The Scytale Cipher
# ==================
#
# The Scytale (pronounced "SKIT-ah-lee") cipher is a *transposition* cipher
# from ancient Sparta (~700 BCE). Unlike substitution ciphers (Caesar, Atbash)
# which replace characters, transposition ciphers rearrange the positions of
# characters without changing them.
#
# How Encryption Works
# --------------------
#
# Given plaintext and a key (number of columns):
#
# 1. Write text row-by-row into a grid with `key` columns.
# 2. Pad the last row with spaces if needed.
# 3. Read the grid column-by-column to produce ciphertext.
#
# Example: encrypt("HELLO WORLD", 3)
#
#     Grid (4 rows x 3 cols):
#         H E L
#         L O ' '
#         W O R
#         L D ' '
#
#     Columns: HLWL + EOOD + L R  = "HLWLEOODL R "
#
# How Decryption Works
# --------------------
#
# 1. Calculate rows = ceil(len / key).
# 2. Write ciphertext column-by-column into the grid.
# 3. Read row-by-row and strip trailing padding spaces.

module CodingAdventures
  module ScytaleCipher
    # Encrypt text using the Scytale transposition cipher.
    #
    # @param text [String] the plaintext to encrypt
    # @param key [Integer] number of columns (>= 2, <= text length)
    # @return [String] the transposed ciphertext
    # @raise [ArgumentError] if key is out of valid range
    def self.encrypt(text, key)
      return "" if text.empty?
      raise ArgumentError, "Key must be >= 2, got #{key}" if key < 2
      raise ArgumentError, "Key must be <= text length (#{text.length}), got #{key}" if key > text.length

      # Calculate grid dimensions and pad
      num_rows = (text.length.to_f / key).ceil
      padded = text.ljust(num_rows * key)

      # Read column-by-column
      result = +""
      key.times do |col|
        num_rows.times do |row|
          result << padded[row * key + col]
        end
      end

      result
    end

    # Decrypt ciphertext that was encrypted with the Scytale cipher.
    #
    # @param text [String] the ciphertext to decrypt
    # @param key [Integer] number of columns used during encryption
    # @return [String] the decrypted plaintext (trailing padding stripped)
    # @raise [ArgumentError] if key is out of valid range
    def self.decrypt(text, key)
      return "" if text.empty?
      raise ArgumentError, "Key must be >= 2, got #{key}" if key < 2
      raise ArgumentError, "Key must be <= text length (#{text.length}), got #{key}" if key > text.length

      n = text.length
      num_rows = (n.to_f / key).ceil

      # Handle uneven grids (when n % key != 0, e.g. during brute-force)
      full_cols = n % key == 0 ? key : n % key

      # Compute column start indices and lengths
      col_starts = []
      col_lens = []
      offset = 0
      key.times do |c|
        col_starts << offset
        len = (n % key == 0 || c < full_cols) ? num_rows : num_rows - 1
        col_lens << len
        offset += len
      end

      # Read row-by-row
      result = +""
      num_rows.times do |row|
        key.times do |col|
          result << text[col_starts[col] + row] if row < col_lens[col]
        end
      end

      result.rstrip
    end

    # Try all possible keys and return decryption results.
    #
    # @param text [String] the ciphertext to brute-force
    # @return [Array<Hash>] list of {key:, text:} results
    def self.brute_force(text)
      return [] if text.length < 4

      max_key = text.length / 2
      (2..max_key).map do |candidate_key|
        { key: candidate_key, text: decrypt(text, candidate_key) }
      end
    end
  end
end
