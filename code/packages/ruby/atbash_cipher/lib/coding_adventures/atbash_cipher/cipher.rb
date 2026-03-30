# frozen_string_literal: true

module CodingAdventures
  # The Atbash cipher is one of the oldest known substitution ciphers,
  # originally used with the Hebrew alphabet. The name "Atbash" derives
  # from the first, last, second, and second-to-last letters of the Hebrew
  # alphabet: Aleph-Tav-Beth-Shin.
  #
  # == How It Works
  #
  # The cipher reverses the alphabet: A maps to Z, B maps to Y, C maps to X,
  # and so on. The complete mapping looks like this:
  #
  #   Plain:  A B C D E F G H I J K L M N O P Q R S T U V W X Y Z
  #   Cipher: Z Y X W V U T S R Q P O N M L K J I H G F E D C B A
  #
  # == The Formula
  #
  # Given a letter at position +p+ (where A=0, B=1, ..., Z=25):
  #
  #   encrypted_position = 25 - p
  #
  # For example, 'H' is at position 7: 25 - 7 = 18, which is 'S'.
  #
  # == Self-Inverse Property
  #
  # The Atbash cipher is *self-inverse*: applying it twice returns the original.
  #
  #   f(f(x)) = 25 - (25 - x) = x
  #
  # This means +encrypt+ and +decrypt+ are the same operation.
  #
  # == Case Preservation
  #
  # Uppercase letters produce uppercase results; lowercase produce lowercase.
  # Non-alphabetic characters (digits, punctuation, spaces) pass through
  # unchanged.
  module AtbashCipher
    # Apply the Atbash substitution to a single character.
    #
    # The algorithm:
    # 1. Check if the character is a letter (A-Z or a-z).
    # 2. If not, return it unchanged.
    # 3. If it is a letter, find its position in the alphabet (0-25).
    # 4. Compute the reversed position: 25 - position.
    # 5. Convert back to a character, preserving case.
    #
    # @param char [String] a single character
    # @return [String] the Atbash-transformed character
    def self.atbash_char(char)
      code = char.ord

      # Uppercase letters: A=65, B=66, ..., Z=90
      if code >= 65 && code <= 90
        position = code - 65         # A=0, B=1, ..., Z=25
        new_position = 25 - position # Reverse the position
        return (65 + new_position).chr
      end

      # Lowercase letters: a=97, b=98, ..., z=122
      if code >= 97 && code <= 122
        position = code - 97         # a=0, b=1, ..., z=25
        new_position = 25 - position # Reverse the position
        return (97 + new_position).chr
      end

      # Non-alphabetic characters pass through unchanged
      char
    end

    # Encrypt text using the Atbash cipher.
    #
    # Each letter is replaced by its reverse in the alphabet (A<->Z, B<->Y,
    # etc.). Non-alphabetic characters pass through unchanged. Case is
    # preserved.
    #
    # Because the Atbash cipher is self-inverse, this method is identical
    # to {decrypt}. Both are provided for API clarity.
    #
    # @param text [String] the plaintext to encrypt
    # @return [String] the encrypted text
    #
    # @example Basic encryption
    #   CodingAdventures::AtbashCipher.encrypt("HELLO")
    #   #=> "SVOOL"
    #
    # @example Case preservation
    #   CodingAdventures::AtbashCipher.encrypt("Hello, World! 123")
    #   #=> "Svool, Dliow! 123"
    def self.encrypt(text)
      text.chars.map { |c| atbash_char(c) }.join
    end

    # Decrypt text using the Atbash cipher.
    #
    # Because the Atbash cipher is self-inverse (applying it twice returns
    # the original), decryption is identical to encryption.
    #
    # @param text [String] the ciphertext to decrypt
    # @return [String] the decrypted text
    #
    # @example
    #   CodingAdventures::AtbashCipher.decrypt("SVOOL")
    #   #=> "HELLO"
    def self.decrypt(text)
      # Decryption IS encryption for Atbash.
      # Proof: f(f(x)) = 25 - (25 - x) = x
      encrypt(text)
    end
  end
end
