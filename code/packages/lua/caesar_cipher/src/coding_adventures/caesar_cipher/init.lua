-- ============================================================================
-- Caesar Cipher — The Oldest Substitution Cipher
-- ============================================================================
--
-- History
-- -------
-- The Caesar cipher is named after Julius Caesar, who used it to protect
-- military messages around 58 BC. According to the Roman historian Suetonius,
-- Caesar shifted each letter in his messages by three positions: A became D,
-- B became E, and so on. When the shift reached the end of the alphabet, it
-- wrapped around — X became A, Y became B, Z became C.
--
-- This makes the Caesar cipher a **monoalphabetic substitution cipher**: every
-- occurrence of a given letter maps to the same replacement letter throughout
-- the entire message. It is the simplest member of a family of ciphers that
-- includes the Vigenere cipher (which uses multiple shifts) and the general
-- substitution cipher (which uses an arbitrary permutation of the alphabet).
--
-- How It Works
-- ------------
-- The cipher operates on the 26 letters of the English alphabet. Given a
-- shift value `s` (also called the "key"), each letter is replaced by the
-- letter `s` positions later in the alphabet, wrapping around at Z.
--
-- Mathematically, if we number the letters A=0, B=1, ..., Z=25:
--
--     encrypt(letter, shift) = (letter + shift) mod 26
--     decrypt(letter, shift) = (letter - shift) mod 26
--
-- Non-alphabetic characters (digits, spaces, punctuation) pass through
-- unchanged. Letter case is preserved: if the input letter is uppercase, the
-- output letter is also uppercase.
--
-- Example with shift=3:
--
--     Plaintext:  HELLO WORLD
--     Ciphertext: KHOOR ZRUOG
--
--     H(7)  + 3 = K(10)
--     E(4)  + 3 = H(7)
--     L(11) + 3 = O(14)
--     L(11) + 3 = O(14)
--     O(14) + 3 = R(17)
--     (space passes through)
--     W(22) + 3 = Z(25)
--     O(14) + 3 = R(17)
--     R(17) + 3 = U(20)
--     L(11) + 3 = O(14)
--     D(3)  + 3 = G(6)
--
-- Breaking the Cipher
-- -------------------
-- Because there are only 25 possible non-trivial shifts (shift=0 is the
-- identity), the Caesar cipher is trivially breakable by **brute force**:
-- just try all 25 shifts and read the results. A human can spot the correct
-- plaintext in seconds.
--
-- A smarter approach uses **frequency analysis**. In English text, letters
-- appear with known frequencies — E is the most common (~12.7%), followed
-- by T (~9.1%), A (~8.2%), and so on. By comparing the frequency
-- distribution of the ciphertext against the expected English frequencies,
-- we can identify the most likely shift without trying all 25.
--
-- We use the **chi-squared statistic** to measure how well the observed
-- letter counts (after applying a candidate shift) match the expected
-- English frequencies. The shift that produces the lowest chi-squared
-- value is the most likely key.
--
-- ============================================================================
--
-- Usage:
--
--   local caesar = require("coding_adventures.caesar_cipher")
--
--   -- Encrypt and decrypt
--   local encrypted = caesar.encrypt("Hello, World!", 3)   --> "Khoor, Zruog!"
--   local decrypted = caesar.decrypt(encrypted, 3)          --> "Hello, World!"
--
--   -- ROT13 (self-inverse: apply twice to get back the original)
--   local secret = caesar.rot13("Hello")                    --> "Uryyb"
--   local back   = caesar.rot13(secret)                     --> "Hello"
--
--   -- Brute-force: try all 25 shifts
--   local results = caesar.brute_force("Khoor")
--   -- results[3].plaintext == "Hello"
--
--   -- Frequency analysis: find the most likely shift
--   local best = caesar.frequency_analysis("some long ciphertext...")
--   -- best.shift, best.plaintext
--
-- ============================================================================

local M = {}

M.VERSION = "0.1.0"

-- ============================================================================
-- English Letter Frequencies
-- ============================================================================
-- These are the expected relative frequencies (as percentages) of each letter
-- in a large sample of English text. They come from classical cryptanalysis
-- references and are used by the frequency_analysis function to score how
-- "English-like" a candidate decryption looks.
--
-- The table is indexed by uppercase letter (A-Z). The percentages sum to
-- approximately 100.

M.ENGLISH_FREQUENCIES = {
    A =  8.167, B =  1.492, C =  2.782, D =  4.253,
    E = 12.702, F =  2.228, G =  2.015, H =  6.094,
    I =  6.966, J =  0.153, K =  0.772, L =  4.025,
    M =  2.406, N =  6.749, O =  7.507, P =  1.929,
    Q =  0.095, R =  5.987, S =  6.327, T =  9.056,
    U =  2.758, V =  0.978, W =  2.360, X =  0.150,
    Y =  1.974, Z =  0.074,
}

-- ============================================================================
-- Helper: ASCII Code Points
-- ============================================================================
-- Lua's string library works with bytes, not characters in the Unicode sense.
-- For the 26 English letters this is fine — they occupy the ASCII range:
--
--     A=65, B=66, ..., Z=90
--     a=97, b=98, ..., z=122
--
-- We cache the byte values of 'A', 'Z', 'a', 'z' so we don't recompute them
-- on every character. string.byte("A") returns 65, etc.

local BYTE_A = string.byte("A")  -- 65
local BYTE_Z = string.byte("Z")  -- 90
local BYTE_a = string.byte("a")  -- 97
local BYTE_z = string.byte("z")  -- 122

-- ============================================================================
-- Helper: shift_char(byte, shift)
-- ============================================================================
-- Given the byte value of a single character and a shift amount, returns the
-- byte value of the shifted character. If the byte is not a letter (A-Z or
-- a-z), it is returned unchanged.
--
-- The key arithmetic:
--   1. Subtract the base ('A' for uppercase, 'a' for lowercase) to get a
--      position in 0..25.
--   2. Add the shift.
--   3. Take modulo 26 to wrap around the alphabet.
--   4. Add the base back to get a valid ASCII code.
--
-- We use (pos + shift) % 26, but Lua's modulo operator handles negative
-- numbers correctly for our purposes: (-1) % 26 == 25 in Lua 5.4, which
-- is exactly the wrap-around behavior we want.

local function shift_char(byte, shift)
    if byte >= BYTE_A and byte <= BYTE_Z then
        -- Uppercase letter: normalize to 0..25, shift, wrap, denormalize
        return ((byte - BYTE_A + shift) % 26) + BYTE_A

    elseif byte >= BYTE_a and byte <= BYTE_z then
        -- Lowercase letter: same logic, different base
        return ((byte - BYTE_a + shift) % 26) + BYTE_a

    else
        -- Not a letter: pass through unchanged (digits, spaces, punctuation)
        return byte
    end
end

-- ============================================================================
-- encrypt(text, shift)
-- ============================================================================
-- Encrypts `text` using the Caesar cipher with the given `shift`.
--
-- Parameters:
--   text  (string) — the plaintext to encrypt
--   shift (number) — the number of positions to shift each letter (can be
--                     negative; will be normalized to 0..25 via modulo)
--
-- Returns:
--   (string) — the ciphertext
--
-- We iterate over each byte of the input string, apply shift_char, and
-- collect the results into a table. At the end we concatenate them all
-- into a single string using table.concat, which is Lua's idiomatic way
-- to build strings efficiently (avoiding O(n^2) concatenation).

function M.encrypt(text, shift)
    -- Normalize shift to the range 0..25. This handles negative shifts
    -- and shifts larger than 26 gracefully.
    shift = shift % 26

    local result = {}
    for i = 1, #text do
        local byte = string.byte(text, i)
        result[i] = string.char(shift_char(byte, shift))
    end
    return table.concat(result)
end

-- ============================================================================
-- decrypt(text, shift)
-- ============================================================================
-- Decrypts `text` that was encrypted with the given `shift`.
--
-- Decryption is just encryption with the negated shift. If we shifted right
-- by 3 to encrypt, we shift left by 3 (i.e., right by 23) to decrypt.
-- The encrypt function already handles negative shifts via modulo.

function M.decrypt(text, shift)
    return M.encrypt(text, -shift)
end

-- ============================================================================
-- rot13(text)
-- ============================================================================
-- ROT13 is a special case of the Caesar cipher with shift=13. Because the
-- English alphabet has 26 letters and 13 is exactly half of 26, applying
-- ROT13 twice returns the original text:
--
--     rot13(rot13("Hello")) == "Hello"
--
-- This self-inverse property made ROT13 popular on Usenet in the 1980s for
-- hiding spoilers and punchlines — anyone could "decode" by applying the
-- same transformation.
--
-- ROT13 is NOT encryption in any meaningful security sense. It is an
-- obfuscation tool, a toy, and a teaching example.

function M.rot13(text)
    return M.encrypt(text, 13)
end

-- ============================================================================
-- brute_force(ciphertext)
-- ============================================================================
-- Tries all 25 possible non-trivial shifts (1 through 25) and returns a
-- table of results. Each entry is a table with two fields:
--
--   { shift = N, plaintext = "..." }
--
-- The caller can inspect the results to find the correct plaintext by
-- reading each candidate. This is the simplest attack on the Caesar cipher
-- and always works — the only question is which result is meaningful
-- English (or whatever the original language was).
--
-- Returns:
--   (table) — a sequence of 25 tables, one per shift

function M.brute_force(ciphertext)
    local results = {}
    for shift = 1, 25 do
        results[#results + 1] = {
            shift = shift,
            plaintext = M.decrypt(ciphertext, shift),
        }
    end
    return results
end

-- ============================================================================
-- frequency_analysis(ciphertext)
-- ============================================================================
-- Uses the chi-squared statistic to find the most likely Caesar shift for
-- the given ciphertext.
--
-- Algorithm:
--   1. For each candidate shift s in 0..25:
--      a. "Decrypt" the ciphertext using shift s.
--      b. Count how many times each letter A-Z appears.
--      c. Compute the chi-squared statistic comparing observed counts to
--         expected counts (based on ENGLISH_FREQUENCIES).
--   2. Return the shift that produced the lowest chi-squared value.
--
-- The chi-squared statistic for a single letter i is:
--
--     chi2_i = (observed_i - expected_i)^2 / expected_i
--
-- where:
--   observed_i = number of times letter i appears in the candidate plaintext
--   expected_i = (total_letters * english_frequency_i) / 100
--
-- Summing over all 26 letters gives the total chi-squared value. A lower
-- value means the observed distribution is closer to English.
--
-- Parameters:
--   ciphertext (string) — the encrypted text to analyze
--
-- Returns:
--   (table) — { shift = N, plaintext = "..." } for the best candidate

function M.frequency_analysis(ciphertext)
    local best_shift = 0
    local best_chi2 = math.huge  -- Start with infinity; we want the minimum
    local best_plaintext = ciphertext

    for shift = 0, 25 do
        local candidate = M.decrypt(ciphertext, shift)

        -- Count letters in the candidate plaintext.
        -- We build a table mapping uppercase letters to their counts.
        local counts = {}
        local total = 0
        for i = 1, #candidate do
            local byte = string.byte(candidate, i)
            local ch = nil
            if byte >= BYTE_A and byte <= BYTE_Z then
                ch = string.char(byte)
            elseif byte >= BYTE_a and byte <= BYTE_z then
                -- Normalize lowercase to uppercase for counting
                ch = string.char(byte - BYTE_a + BYTE_A)
            end
            if ch then
                counts[ch] = (counts[ch] or 0) + 1
                total = total + 1
            end
        end

        -- If there are no letters at all, chi-squared is meaningless.
        -- We treat this as a perfect score (0) so shift=0 wins by default.
        if total == 0 then
            if shift == 0 then
                best_shift = 0
                best_chi2 = 0
                best_plaintext = candidate
            end
        else
            -- Compute chi-squared against English frequencies.
            local chi2 = 0
            for letter, freq in pairs(M.ENGLISH_FREQUENCIES) do
                local observed = counts[letter] or 0
                local expected = (total * freq) / 100.0
                -- Guard against division by zero (some letters like Z have
                -- very low expected counts, but never exactly zero since
                -- freq > 0 for all letters).
                if expected > 0 then
                    chi2 = chi2 + ((observed - expected) ^ 2) / expected
                end
            end

            if chi2 < best_chi2 then
                best_chi2 = chi2
                best_shift = shift
                best_plaintext = candidate
            end
        end
    end

    return {
        shift = best_shift,
        plaintext = best_plaintext,
    }
end

-- ============================================================================
-- Module Export
-- ============================================================================
-- We return a single table containing all public functions and constants.
-- This follows Lua's module convention: require("module") returns a table.

return M
