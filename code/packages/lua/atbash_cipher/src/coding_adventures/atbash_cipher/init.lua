-- ============================================================================
-- CodingAdventures.AtbashCipher
-- ============================================================================
--
-- The Atbash cipher: a fixed reverse-alphabet substitution cipher.
--
-- What is the Atbash Cipher?
-- --------------------------
--
-- The Atbash cipher is one of the oldest known substitution ciphers,
-- originally used with the Hebrew alphabet. The name "Atbash" comes from
-- the first, last, second, and second-to-last letters of the Hebrew
-- alphabet: Aleph-Tav-Beth-Shin.
--
-- The cipher reverses the alphabet:
--
--     Plain:  A B C D E F G H I J K L M N O P Q R S T U V W X Y Z
--     Cipher: Z Y X W V U T S R Q P O N M L K J I H G F E D C B A
--
-- The Formula
-- -----------
--
-- Given a letter at position p (where A=0, B=1, ..., Z=25):
--
--     encrypted_position = 25 - p
--
-- For example, 'H' is at position 7: 25 - 7 = 18, which is 'S'.
--
-- Self-Inverse Property
-- ---------------------
--
-- The Atbash cipher is self-inverse: applying it twice returns the original.
--
--     f(f(x)) = 25 - (25 - x) = x
--
-- This means encrypt() and decrypt() are the same operation.
--
-- Usage:
--
--   local atbash = require("coding_adventures.atbash_cipher")
--   print(atbash.encrypt("HELLO"))  -- "SVOOL"
--   print(atbash.decrypt("SVOOL"))  -- "HELLO"
--
-- ============================================================================

local M = {}

M.VERSION = "0.1.0"

-- ASCII code points for reference:
-- A = 65, Z = 90
-- a = 97, z = 122

local UPPER_A = string.byte("A")  -- 65
local UPPER_Z = string.byte("Z")  -- 90
local LOWER_A = string.byte("a")  -- 97
local LOWER_Z = string.byte("z")  -- 122

--- Apply the Atbash substitution to a single byte value.
--
-- The algorithm:
-- 1. Check if the byte is an uppercase (65-90) or lowercase (97-122) letter.
-- 2. If it's a letter, compute its position (0-25), reverse it (25 - pos),
--    and convert back to a byte value.
-- 3. If it's not a letter, return it unchanged.
--
-- @param byte_val number  The byte value of the character
-- @return number  The Atbash-transformed byte value
local function atbash_byte(byte_val)
    -- Uppercase letters: A(65) through Z(90)
    if byte_val >= UPPER_A and byte_val <= UPPER_Z then
        local pos = byte_val - UPPER_A      -- A=0, B=1, ..., Z=25
        local new_pos = 25 - pos            -- Reverse: 0->25, 1->24, ..., 25->0
        return UPPER_A + new_pos            -- Convert back
    end

    -- Lowercase letters: a(97) through z(122)
    if byte_val >= LOWER_A and byte_val <= LOWER_Z then
        local pos = byte_val - LOWER_A      -- a=0, b=1, ..., z=25
        local new_pos = 25 - pos            -- Reverse
        return LOWER_A + new_pos            -- Convert back
    end

    -- Non-alphabetic bytes pass through unchanged
    return byte_val
end

--- Encrypt text using the Atbash cipher.
--
-- Each letter is replaced by its reverse in the alphabet (A<->Z, B<->Y, etc.).
-- Non-alphabetic characters pass through unchanged. Case is preserved.
--
-- Because the Atbash cipher is self-inverse, this function is identical
-- to decrypt(). Both are provided for API clarity.
--
-- @param text string  The plaintext to encrypt
-- @return string  The encrypted text
function M.encrypt(text)
    -- Process each byte in the string, apply Atbash, collect results.
    -- We use string.byte to get the byte values and string.char to
    -- convert back. This works correctly for ASCII text.
    local result = {}
    for i = 1, #text do
        local byte_val = string.byte(text, i)
        result[i] = string.char(atbash_byte(byte_val))
    end
    return table.concat(result)
end

--- Decrypt text using the Atbash cipher.
--
-- Because the Atbash cipher is self-inverse (applying it twice returns
-- the original), decryption is identical to encryption. This function
-- exists for API clarity.
--
-- @param text string  The ciphertext to decrypt
-- @return string  The decrypted text
function M.decrypt(text)
    -- Decryption IS encryption for Atbash.
    -- Proof: f(f(x)) = 25 - (25 - x) = x
    return M.encrypt(text)
end

return M
