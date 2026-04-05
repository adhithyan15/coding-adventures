-- ============================================================================
-- CodingAdventures.ScytaleCipher
-- ============================================================================
--
-- The Scytale (pronounced "SKIT-ah-lee") cipher is a *transposition* cipher
-- from ancient Sparta (~700 BCE). Unlike substitution ciphers (Caesar, Atbash)
-- which replace characters, the Scytale rearranges character positions using
-- a columnar transposition.
--
-- How Encryption Works
-- --------------------
--
-- 1. Write text row-by-row into a grid with `key` columns.
-- 2. Pad the last row with spaces if needed.
-- 3. Read column-by-column to produce ciphertext.
--
-- Example: encrypt("HELLO WORLD", 3)
--
--     Grid (4 rows x 3 cols):
--         H E L
--         L O ' '
--         W O R
--         L D ' '
--
--     Columns: HLWL + EOOD + L R  = "HLWLEOODL R "
--
-- How Decryption Works
-- --------------------
--
-- 1. Calculate rows = ceil(len / key).
-- 2. Write ciphertext column-by-column.
-- 3. Read row-by-row and strip trailing padding spaces.
--
-- Why It's Insecure
-- -----------------
--
-- The key space is tiny: for a message of length n, there are only
-- about n/2 possible keys. BruteForce demonstrates this.

local M = {}

--- Encrypt text using the Scytale transposition cipher.
--- @param text string The plaintext to encrypt
--- @param key number Number of columns (>= 2, <= #text)
--- @return string The transposed ciphertext
function M.encrypt(text, key)
    if text == "" then return "" end
    local n = #text
    assert(key >= 2, "Key must be >= 2, got " .. tostring(key))
    assert(key <= n, "Key must be <= text length (" .. n .. "), got " .. tostring(key))

    -- Calculate grid dimensions and pad
    local num_rows = math.ceil(n / key)
    local padded_len = num_rows * key
    local padded = text .. string.rep(" ", padded_len - n)

    -- Read column-by-column
    local result = {}
    for col = 0, key - 1 do
        for row = 0, num_rows - 1 do
            local idx = row * key + col + 1  -- Lua is 1-indexed
            result[#result + 1] = padded:sub(idx, idx)
        end
    end

    return table.concat(result)
end

--- Decrypt ciphertext encrypted with the Scytale cipher.
--- Trailing padding spaces are stripped.
--- @param text string The ciphertext to decrypt
--- @param key number Number of columns used during encryption
--- @return string The decrypted plaintext
function M.decrypt(text, key)
    if text == "" then return "" end
    local n = #text
    assert(key >= 2, "Key must be >= 2, got " .. tostring(key))
    assert(key <= n, "Key must be <= text length (" .. n .. "), got " .. tostring(key))

    local num_rows = math.ceil(n / key)

    -- Handle uneven grids (when n % key != 0, e.g. during brute-force)
    local full_cols = n % key == 0 and key or (n % key)

    -- Compute column start indices and lengths
    local col_starts = {}
    local col_lens = {}
    local offset = 0
    for c = 0, key - 1 do
        col_starts[c] = offset
        local col_len
        if n % key == 0 or c < full_cols then
            col_len = num_rows
        else
            col_len = num_rows - 1
        end
        col_lens[c] = col_len
        offset = offset + col_len
    end

    -- Read row-by-row
    local result = {}
    for row = 0, num_rows - 1 do
        for col = 0, key - 1 do
            if row < col_lens[col] then
                local idx = col_starts[col] + row + 1  -- Lua is 1-indexed
                result[#result + 1] = text:sub(idx, idx)
            end
        end
    end

    -- Strip trailing padding spaces
    local joined = table.concat(result)
    return (joined:gsub("%s+$", ""))
end

--- Try all possible keys and return decryption results.
--- @param text string The ciphertext to brute-force
--- @return table List of {key=number, text=string} results
function M.brute_force(text)
    local n = #text
    if n < 4 then return {} end

    local max_key = math.floor(n / 2)
    local results = {}

    for candidate_key = 2, max_key do
        results[#results + 1] = {
            key = candidate_key,
            text = M.decrypt(text, candidate_key),
        }
    end

    return results
end

return M
