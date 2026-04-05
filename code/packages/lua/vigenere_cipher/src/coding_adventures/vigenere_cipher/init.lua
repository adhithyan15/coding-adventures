-- ============================================================================
-- CodingAdventures.VigenereCipher
-- ============================================================================
--
-- The Vigenere cipher is a *polyalphabetic substitution* cipher invented by
-- Giovan Battista Bellaso in 1553 and later misattributed to Blaise de
-- Vigenere. For 300 years it was considered "le chiffre indechiffrable"
-- until Friedrich Kasiski published a general method for breaking it in 1863.
--
-- How It Works (Encryption)
-- -------------------------
--
-- Unlike a Caesar cipher (one fixed shift), the Vigenere cipher uses a
-- *keyword* to apply a different shift at each position:
--
--     Plaintext:  A  T  T  A  C  K  A  T  D  A  W  N
--     Keyword:    L  E  M  O  N  L  E  M  O  N  L  E
--     Shift:      11 4  12 14 13 11 4  12 14 13 11 4
--     Ciphertext: L  X  F  O  P  V  E  F  R  N  H  R
--
-- Each plaintext letter is shifted forward by the amount indicated by the
-- corresponding keyword letter (A=0, B=1, ... Z=25). Non-alphabetic
-- characters pass through unchanged and do NOT advance the keyword position.
--
-- How It Works (Decryption)
-- -------------------------
--
-- Reverse the process: shift each letter *backward* by the keyword amount.
--
-- Cryptanalysis (Breaking the Cipher)
-- ------------------------------------
--
-- Breaking the Vigenere cipher requires two steps:
--
-- Step 1 -- Find the key length using the Index of Coincidence (IC).
-- For each candidate key length k, split the ciphertext into k groups
-- (every k-th letter). Calculate the average IC across groups. English
-- text has IC ~ 0.0667; random text ~ 0.0385. The key length producing
-- the highest average IC is likely correct.
--
-- Step 2 -- Find each key letter using chi-squared analysis.
-- For each position in the key, extract the group of letters at that
-- position, try all 26 shifts, and pick the shift producing the lowest
-- chi-squared statistic against English letter frequencies.

local M = {}

-- ============================================================================
-- English Letter Frequencies
-- ============================================================================
--
-- These are the expected frequencies of each letter in typical English text,
-- used by the chi-squared test to determine the most likely shift for each
-- key position. Source: standard corpus analysis.

local ENGLISH_FREQ = {
    0.08167, 0.01492, 0.02782, 0.04253, 0.12702, 0.02228, -- A-F
    0.02015, 0.06094, 0.06966, 0.00153, 0.00772, 0.04025, -- G-L
    0.02406, 0.06749, 0.07507, 0.01929, 0.00095, 0.05987, -- M-R
    0.06327, 0.09056, 0.02758, 0.00978, 0.02360, 0.00150, -- S-X
    0.01974, 0.00074,                                       -- Y-Z
}

-- ============================================================================
-- Helper: Check if a character code represents an ASCII letter
-- ============================================================================

local function is_alpha(byte)
    return (byte >= 65 and byte <= 90) or (byte >= 97 and byte <= 122)
end

-- ============================================================================
-- Helper: Check if a character code is uppercase
-- ============================================================================

local function is_upper(byte)
    return byte >= 65 and byte <= 90
end

-- ============================================================================
-- encrypt(plaintext, key) -> string
-- ============================================================================
--
-- Encrypt plaintext using the Vigenere cipher with the given key.
--
-- Rules:
--   * Key must be non-empty and contain only A-Z / a-z.
--   * Uppercase letters stay uppercase; lowercase stay lowercase.
--   * Non-alphabetic characters pass through unchanged.
--   * The key position advances only on alphabetic characters.
--
-- Example:
--   encrypt("ATTACKATDAWN", "LEMON") --> "LXFOPVEFRNHR"
--   encrypt("Hello, World!", "key")  --> "Rijvs, Uyvjn!"

function M.encrypt(plaintext, key)
    assert(type(key) == "string" and #key > 0, "Key must be a non-empty string")
    -- Validate key contains only letters
    for i = 1, #key do
        assert(is_alpha(key:byte(i)), "Key must contain only letters, got '" .. key .. "'")
    end

    local result = {}
    local key_len = #key
    local key_idx = 0  -- 0-based index into the key

    for i = 1, #plaintext do
        local p = plaintext:byte(i)

        if is_alpha(p) then
            -- Determine the shift amount from the current key letter.
            -- We normalize the key letter to 0-25 regardless of its case.
            local k = key:byte((key_idx % key_len) + 1)
            local shift = (is_upper(k)) and (k - 65) or (k - 97)

            -- Apply the shift, preserving the case of the plaintext letter.
            local base = is_upper(p) and 65 or 97
            local shifted = (p - base + shift) % 26 + base
            result[#result + 1] = string.char(shifted)

            key_idx = key_idx + 1
        else
            -- Non-alpha passes through; key does NOT advance.
            result[#result + 1] = string.char(p)
        end
    end

    return table.concat(result)
end

-- ============================================================================
-- decrypt(ciphertext, key) -> string
-- ============================================================================
--
-- Decrypt ciphertext by shifting each letter *backward* by the key amount.
-- This is the exact inverse of encrypt.
--
-- Example:
--   decrypt("LXFOPVEFRNHR", "LEMON") --> "ATTACKATDAWN"

function M.decrypt(ciphertext, key)
    assert(type(key) == "string" and #key > 0, "Key must be a non-empty string")
    for i = 1, #key do
        assert(is_alpha(key:byte(i)), "Key must contain only letters, got '" .. key .. "'")
    end

    local result = {}
    local key_len = #key
    local key_idx = 0

    for i = 1, #ciphertext do
        local c = ciphertext:byte(i)

        if is_alpha(c) then
            local k = key:byte((key_idx % key_len) + 1)
            local shift = is_upper(k) and (k - 65) or (k - 97)

            -- Shift backward (subtract), add 26 to avoid negative modulo.
            local base = is_upper(c) and 65 or 97
            local shifted = (c - base - shift + 26) % 26 + base
            result[#result + 1] = string.char(shifted)

            key_idx = key_idx + 1
        else
            result[#result + 1] = string.char(c)
        end
    end

    return table.concat(result)
end

-- ============================================================================
-- index_of_coincidence(text) -> number
-- ============================================================================
--
-- The Index of Coincidence (IC) measures how likely it is that two randomly
-- chosen letters from a text are the same. For English, IC ~ 0.0667.
-- For random uniform text, IC ~ 1/26 ~ 0.0385.
--
-- Formula: IC = sum(n_i * (n_i - 1)) / (N * (N - 1))
-- where n_i is the count of the i-th letter and N is the total letter count.

local function index_of_coincidence(text)
    local counts = {}
    for i = 0, 25 do counts[i] = 0 end

    local total = 0
    for i = 1, #text do
        local b = text:byte(i)
        if is_alpha(b) then
            local idx = is_upper(b) and (b - 65) or (b - 97)
            counts[idx] = counts[idx] + 1
            total = total + 1
        end
    end

    if total <= 1 then return 0 end

    local sum = 0
    for i = 0, 25 do
        sum = sum + counts[i] * (counts[i] - 1)
    end

    return sum / (total * (total - 1))
end

-- ============================================================================
-- find_key_length(ciphertext, max_length) -> number
-- ============================================================================
--
-- Estimate the key length of a Vigenere-encrypted ciphertext using
-- Index of Coincidence analysis.
--
-- Algorithm:
--   For each candidate key length k (from 2 to max_length):
--     1. Split ciphertext into k groups (every k-th letter).
--     2. Compute IC of each group.
--     3. Average the ICs.
--   The key length with the highest average IC is most likely correct,
--   because at the correct key length each group is a simple Caesar cipher
--   of English text (IC ~ 0.0667).

function M.find_key_length(ciphertext, max_length)
    max_length = max_length or 20

    -- Extract only alphabetic characters for analysis
    local alpha_only = {}
    for i = 1, #ciphertext do
        local b = ciphertext:byte(i)
        if is_alpha(b) then
            alpha_only[#alpha_only + 1] = string.char(b)
        end
    end
    local alpha_str = table.concat(alpha_only)
    local n = #alpha_str

    if n < 2 then return 1 end

    local best_length = 1
    local best_ic = -1

    for k = 2, math.min(max_length, math.floor(n / 2)) do
        -- Split into k groups: group j gets characters at positions j, j+k, j+2k, ...
        local ic_sum = 0
        for j = 1, k do
            local group = {}
            local pos = j
            while pos <= n do
                group[#group + 1] = alpha_str:sub(pos, pos)
                pos = pos + k
            end
            ic_sum = ic_sum + index_of_coincidence(table.concat(group))
        end

        local avg_ic = ic_sum / k
        if avg_ic > best_ic then
            best_ic = avg_ic
            best_length = k
        end
    end

    return best_length
end

-- ============================================================================
-- chi_squared(observed_counts, total, expected_freq) -> number
-- ============================================================================
--
-- The chi-squared statistic measures how well observed letter frequencies
-- match expected English frequencies. Lower values mean a better fit.
--
-- Formula: chi2 = sum( (observed_i - expected_i)^2 / expected_i )
-- where expected_i = total * expected_freq[i]

local function chi_squared(counts, total, expected)
    local chi2 = 0
    for i = 0, 25 do
        local exp = total * expected[i + 1]
        if exp > 0 then
            local diff = counts[i] - exp
            chi2 = chi2 + (diff * diff) / exp
        end
    end
    return chi2
end

-- ============================================================================
-- find_key(ciphertext, key_length) -> string
-- ============================================================================
--
-- Given a ciphertext and known key length, find the key by chi-squared
-- analysis on each position.
--
-- For each key position (0..key_length-1):
--   1. Extract the group of letters at that position.
--   2. Try all 26 possible shifts (A=0, B=1, ..., Z=25).
--   3. For each shift, compute frequency counts of the shifted letters.
--   4. The shift with the lowest chi-squared against English is the key letter.

function M.find_key(ciphertext, key_length)
    -- Extract only alpha characters
    local alpha_only = {}
    for i = 1, #ciphertext do
        local b = ciphertext:byte(i)
        if is_alpha(b) then
            alpha_only[#alpha_only + 1] = is_upper(b) and (b - 65) or (b - 97)
        end
    end
    local n = #alpha_only

    local key_chars = {}

    for pos = 1, key_length do
        -- Gather the letters at this key position (every key_length-th letter)
        local group = {}
        local idx = pos
        while idx <= n do
            group[#group + 1] = alpha_only[idx]
            idx = idx + key_length
        end

        local group_size = #group
        if group_size == 0 then
            key_chars[#key_chars + 1] = "A"
        else
            -- Try all 26 shifts and pick the one with lowest chi-squared
            local best_shift = 0
            local best_chi2 = math.huge

            for shift = 0, 25 do
                local counts = {}
                for i = 0, 25 do counts[i] = 0 end

                for _, val in ipairs(group) do
                    local decrypted = (val - shift + 26) % 26
                    counts[decrypted] = counts[decrypted] + 1
                end

                local chi2 = chi_squared(counts, group_size, ENGLISH_FREQ)
                if chi2 < best_chi2 then
                    best_chi2 = chi2
                    best_shift = shift
                end
            end

            key_chars[#key_chars + 1] = string.char(65 + best_shift)
        end
    end

    return table.concat(key_chars)
end

-- ============================================================================
-- break_cipher(ciphertext) -> key, plaintext
-- ============================================================================
--
-- Automatic Vigenere cipher break. Combines find_key_length and find_key
-- to recover the key and plaintext without any prior knowledge.
--
-- Returns two values: the recovered key (uppercase) and the decrypted text.

function M.break_cipher(ciphertext)
    local key_length = M.find_key_length(ciphertext)
    local key = M.find_key(ciphertext, key_length)
    local plaintext = M.decrypt(ciphertext, key)
    return key, plaintext
end

return M
