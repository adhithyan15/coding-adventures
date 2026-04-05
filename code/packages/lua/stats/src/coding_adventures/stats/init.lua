-- ============================================================================
-- Statistics -- Descriptive Statistics, Frequency Analysis, and Cryptanalysis
-- ============================================================================
--
-- Overview
-- --------
-- This module provides three categories of pure functions:
--
-- 1. **Descriptive statistics** (mean, median, mode, variance, standard
--    deviation, min, max, range) -- operate on arrays of numbers.
-- 2. **Frequency analysis** (frequency_count, frequency_distribution,
--    chi_squared, chi_squared_text) -- operate on text strings or arrays.
-- 3. **Cryptanalysis helpers** (index_of_coincidence, entropy,
--    ENGLISH_FREQUENCIES) -- tools for breaking classical ciphers.
--
-- Design Principles
-- -----------------
-- - **Pure functions.** No side effects, no mutation of inputs.
-- - **No external dependencies.** Pure math only.
-- - **Population vs sample.** Variance and standard deviation default to
--   sample (Bessel-corrected, dividing by n-1). Pass population=true for
--   population statistics (dividing by n).
--
-- Worked Example
-- --------------
-- Given the dataset {2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0}:
--
--   mean     = (2+4+4+4+5+5+7+9) / 8 = 40 / 8 = 5.0
--   median   = average of 4th and 5th values (sorted) = (4+5)/2 = 4.5
--   mode     = 4.0 (appears 3 times, more than any other)
--   variance = sample: sum of squared deviations / (n-1)
--            = [(2-5)^2 + (4-5)^2 + ... + (9-5)^2] / 7
--            = 32 / 7 = 4.571428...
--   std_dev  = sqrt(4.571428...) = 2.138...
--   min      = 2.0
--   max      = 9.0
--   range    = 9.0 - 2.0 = 7.0
-- ============================================================================

local Stats = {}

-- ============================================================================
-- English Letter Frequencies
-- ============================================================================
--
-- These are the standard frequencies of each letter in English text, derived
-- from large-corpus analysis. Index by uppercase letter. Used as the expected
-- distribution for chi-squared tests in frequency analysis and cryptanalysis.
--
--   A = 0.08167  (about 8.2% of English text)
--   E = 0.12702  (the most common letter at ~12.7%)
--   Z = 0.00074  (the least common letter at ~0.07%)
--
-- The frequencies sum to approximately 1.0 (they represent proportions,
-- not percentages).
-- ============================================================================

Stats.ENGLISH_FREQUENCIES = {
    A = 0.08167, B = 0.01492, C = 0.02782, D = 0.04253,
    E = 0.12702, F = 0.02228, G = 0.02015, H = 0.06094,
    I = 0.06966, J = 0.00153, K = 0.00772, L = 0.04025,
    M = 0.02406, N = 0.06749, O = 0.07507, P = 0.01929,
    Q = 0.00095, R = 0.05987, S = 0.06327, T = 0.09056,
    U = 0.02758, V = 0.00978, W = 0.02360, X = 0.00150,
    Y = 0.01974, Z = 0.00074,
}

-- ============================================================================
-- Descriptive Statistics
-- ============================================================================

--- Arithmetic mean: sum of all values divided by the count.
--
-- The mean is the most common measure of central tendency. It uses every
-- data point, which makes it sensitive to outliers.
--
-- Formula: mean = (x_1 + x_2 + ... + x_n) / n
--
-- Example:
--   mean({1, 2, 3, 4, 5}) = 15 / 5 = 3.0
--
-- @param values  A table (array) of numbers.
-- @return The arithmetic mean as a number.
function Stats.mean(values)
    assert(#values > 0, "mean requires at least one value")
    local total = 0
    for _, v in ipairs(values) do
        total = total + v
    end
    return total / #values
end

--- Median: the middle value when sorted.
--
-- The median splits the dataset in half -- 50% of values are below it and
-- 50% are above. Unlike the mean, the median is robust to outliers.
--
-- For odd-length lists, the median is the middle element.
-- For even-length lists, it is the average of the two middle elements.
--
-- Examples:
--   median({1, 2, 3, 4, 5})  -> 3.0   (middle of 5 elements)
--   median({1, 2, 3, 4})     -> 2.5   (average of 2 and 3)
--
-- @param values  A table (array) of numbers.
-- @return The median value as a number.
function Stats.median(values)
    assert(#values > 0, "median requires at least one value")

    -- Copy and sort (we never mutate the input)
    local sorted = {}
    for i, v in ipairs(values) do
        sorted[i] = v
    end
    table.sort(sorted)

    local n = #sorted
    local mid = math.floor(n / 2)

    -- Odd length: single middle element
    if n % 2 == 1 then
        return sorted[mid + 1]
    end

    -- Even length: average of two middle elements
    return (sorted[mid] + sorted[mid + 1]) / 2.0
end

--- Mode: the most frequently occurring value.
--
-- If multiple values share the highest frequency, the one that appears
-- first in the original list wins. This "first occurrence" tie-breaking
-- rule ensures deterministic results across all languages in the repo.
--
-- How it works:
-- 1. Count occurrences of each value.
-- 2. Find the maximum count.
-- 3. Return the first value in the original list that has that count.
--
-- Example:
--   mode({2, 4, 4, 4, 5, 5, 7, 9}) -> 4.0  (appears 3 times)
--
-- @param values  A table (array) of numbers.
-- @return The mode value as a number.
function Stats.mode(values)
    assert(#values > 0, "mode requires at least one value")

    -- Step 1: count occurrences
    local counts = {}
    for _, v in ipairs(values) do
        counts[v] = (counts[v] or 0) + 1
    end

    -- Step 2: find the maximum frequency
    local max_count = 0
    for _, c in pairs(counts) do
        if c > max_count then
            max_count = c
        end
    end

    -- Step 3: return the first value with that frequency
    for _, v in ipairs(values) do
        if counts[v] == max_count then
            return v
        end
    end
end

--- Variance: average of squared deviations from the mean.
--
-- Variance measures how spread out the data is. A variance of 0 means
-- all values are identical.
--
-- Two flavors:
--   - **Sample variance** (default, population=false): divides by n-1.
--     Used when your data is a sample from a larger population. The n-1
--     correction (Bessel's correction) makes the estimate unbiased.
--   - **Population variance** (population=true): divides by n.
--     Used when your data IS the entire population.
--
-- Formula:
--   variance = Sum((x_i - mean)^2) / d
--   where d = n (population) or n-1 (sample)
--
-- Examples:
--   variance({2,4,4,4,5,5,7,9})                -> 4.571428... (sample)
--   variance({2,4,4,4,5,5,7,9}, true)           -> 4.0        (population)
--
-- @param values     A table (array) of numbers.
-- @param population Boolean. If true, use population variance. Default false.
-- @return The variance as a number.
function Stats.variance(values, population)
    assert(#values > 0, "variance requires at least one value")
    local n = #values

    if not population and n == 1 then
        error("sample variance requires at least two values")
    end

    local m = Stats.mean(values)

    -- Sum of squared deviations
    -- Each (x_i - mean)^2 measures how far that point is from the center.
    local squared_diffs = 0
    for _, v in ipairs(values) do
        squared_diffs = squared_diffs + (v - m) ^ 2
    end

    local divisor = population and n or (n - 1)
    return squared_diffs / divisor
end

--- Standard deviation: square root of variance.
--
-- The standard deviation has the same units as the original data (unlike
-- variance, which is in squared units). This makes it more interpretable.
--
-- For a normal distribution:
--   - ~68% of data falls within 1 standard deviation of the mean
--   - ~95% falls within 2 standard deviations
--   - ~99.7% falls within 3 standard deviations
--
-- @param values     A table (array) of numbers.
-- @param population Boolean. If true, use population std dev. Default false.
-- @return The standard deviation as a number.
function Stats.standard_deviation(values, population)
    return math.sqrt(Stats.variance(values, population))
end

--- Minimum value in the dataset.
--
-- @param values  A table (array) of numbers.
-- @return The smallest value.
function Stats.min(values)
    assert(#values > 0, "min requires at least one value")
    local result = values[1]
    for i = 2, #values do
        if values[i] < result then
            result = values[i]
        end
    end
    return result
end

--- Maximum value in the dataset.
--
-- @param values  A table (array) of numbers.
-- @return The largest value.
function Stats.max(values)
    assert(#values > 0, "max requires at least one value")
    local result = values[1]
    for i = 2, #values do
        if values[i] > result then
            result = values[i]
        end
    end
    return result
end

--- Range: the difference between the maximum and minimum values.
--
-- The range is the simplest measure of spread. It only looks at the two
-- extreme values, so it is very sensitive to outliers.
--
-- Formula: range = max - min
--
-- Example:
--   range({2, 4, 4, 4, 5, 5, 7, 9}) = 9 - 2 = 7.0
--
-- @param values  A table (array) of numbers.
-- @return The range as a number.
function Stats.range(values)
    return Stats.max(values) - Stats.min(values)
end

-- ============================================================================
-- Frequency Analysis
-- ============================================================================
--
-- These functions analyze the frequency distribution of letters in text.
-- They are the foundation of classical cipher analysis: by comparing
-- observed letter frequencies against the known English distribution,
-- we can detect whether a ciphertext was encrypted with a substitution
-- cipher and potentially recover the key.
-- ============================================================================

--- Count each letter in text (case-insensitive, A-Z only).
--
-- Non-alphabetic characters are ignored. The result is a table mapping
-- uppercase letters to their integer counts.
--
-- Example:
--   frequency_count("Hello!") -> {H=1, E=1, L=2, O=1}
--
-- @param text  A string to analyze.
-- @return A table mapping uppercase letters to integer counts.
function Stats.frequency_count(text)
    local counts = {}
    -- Initialize all 26 letters to 0
    for i = 0, 25 do
        counts[string.char(65 + i)] = 0
    end

    local upper = string.upper(text)
    for i = 1, #upper do
        local ch = upper:sub(i, i)
        if ch >= "A" and ch <= "Z" then
            counts[ch] = counts[ch] + 1
        end
    end

    return counts
end

--- Frequency distribution: proportion of each letter in the text.
--
-- Like frequency_count, but returns proportions (counts / total letters)
-- instead of raw counts. This normalizes for text length, allowing
-- comparison between texts of different sizes.
--
-- Example:
--   frequency_distribution("AABB") -> {A=0.5, B=0.5, C=0.0, ...}
--
-- @param text  A string to analyze.
-- @return A table mapping uppercase letters to float proportions.
function Stats.frequency_distribution(text)
    local counts = Stats.frequency_count(text)

    -- Sum total alphabetic characters
    local total = 0
    for _, c in pairs(counts) do
        total = total + c
    end

    local dist = {}
    for letter, count in pairs(counts) do
        dist[letter] = total > 0 and (count / total) or 0.0
    end

    return dist
end

--- Chi-squared goodness-of-fit test for parallel arrays.
--
-- The chi-squared statistic measures how well observed data fits an
-- expected distribution. It is computed as:
--
--   chi2 = Sum((O_i - E_i)^2 / E_i)
--
-- A chi2 of 0 means perfect fit. Larger values mean worse fit.
--
-- Example:
--   chi_squared({10, 20, 30}, {20, 20, 20})
--   = (10-20)^2/20 + (20-20)^2/20 + (30-20)^2/20
--   = 5.0 + 0.0 + 5.0 = 10.0
--
-- @param observed  Array of observed counts.
-- @param expected  Array of expected counts (same length as observed).
-- @return The chi-squared statistic as a number.
function Stats.chi_squared(observed, expected)
    assert(#observed == #expected, "observed and expected must have same length")
    assert(#observed > 0, "arrays must not be empty")

    local chi2 = 0
    for i = 1, #observed do
        if expected[i] > 1e-10 then
            local diff = observed[i] - expected[i]
            chi2 = chi2 + (diff * diff) / expected[i]
        end
    end

    return chi2
end

--- Chi-squared test of text against an expected frequency table.
--
-- This is a convenience function that combines frequency_count with
-- chi_squared. It counts the letters in the text, then compares those
-- counts against the expected frequencies scaled to the text length.
--
-- Typical usage: compare ciphertext candidate against ENGLISH_FREQUENCIES
-- to see if a decryption attempt produces English-like text.
--
-- @param text          A string to analyze.
-- @param expected_freq A table mapping uppercase letters to frequency proportions.
-- @return The chi-squared statistic as a number.
function Stats.chi_squared_text(text, expected_freq)
    local counts = Stats.frequency_count(text)

    -- Total alphabetic characters
    local total = 0
    for _, c in pairs(counts) do
        total = total + c
    end

    if total == 0 then
        return 0
    end

    local chi2 = 0
    for i = 0, 25 do
        local letter = string.char(65 + i)
        local observed = counts[letter] or 0
        local expected = total * (expected_freq[letter] or 0)
        if expected > 1e-10 then
            local diff = observed - expected
            chi2 = chi2 + (diff * diff) / expected
        end
    end

    return chi2
end

-- ============================================================================
-- Cryptanalysis Helpers
-- ============================================================================
--
-- These functions compute metrics that help determine what kind of cipher
-- was used to encrypt a message. They complement the frequency analysis
-- functions above.
--
-- Index of Coincidence (IC):
--   Measures the probability that two randomly chosen letters from the
--   text are the same. English text has IC ~0.0667, while random text
--   has IC ~0.0385 (1/26). A polyalphabetic cipher (like Vigenere)
--   flattens the IC toward the random value.
--
-- Shannon Entropy:
--   Measures the information content (in bits) of the text's letter
--   distribution. Uniform distribution gives maximum entropy (log2(26)
--   ~ 4.700 bits). English text has lower entropy (~4.1 bits) because
--   some letters are much more common than others.
-- ============================================================================

--- Index of Coincidence: probability that two random letters match.
--
-- Formula:
--   IC = Sum(n_i * (n_i - 1)) / (N * (N - 1))
--
-- where n_i is the count of the i-th letter and N is the total number
-- of letters.
--
-- Reference values:
--   - English text:  IC ~ 0.0667
--   - Random text:   IC ~ 0.0385 (1/26)
--   - "AABB":        IC = (2*1 + 2*1) / (4*3) = 4/12 = 0.333...
--
-- @param text  A string to analyze.
-- @return The index of coincidence as a number.
function Stats.index_of_coincidence(text)
    local counts = Stats.frequency_count(text)

    -- N = total alphabetic characters
    local n = 0
    for _, c in pairs(counts) do
        n = n + c
    end

    -- Need at least 2 letters
    if n < 2 then
        return 0.0
    end

    -- Sum(n_i * (n_i - 1))
    local numerator = 0
    for _, c in pairs(counts) do
        numerator = numerator + c * (c - 1)
    end

    return numerator / (n * (n - 1))
end

--- Shannon entropy of the letter distribution in text.
--
-- Entropy measures the "surprise" or information content of a distribution.
-- The formula is:
--
--   H = -Sum(p_i * log2(p_i))
--
-- where p_i is the probability (proportion) of each letter.
--
-- Higher entropy means a more uniform distribution (more randomness).
-- Lower entropy means some letters dominate (less randomness).
--
-- Reference values:
--   - 26 equal letters: H = log2(26) ~ 4.700 bits
--   - English text:     H ~ 4.1 bits
--   - "AAAA":           H = 0.0 bits (no surprise)
--
-- @param text  A string to analyze.
-- @return The Shannon entropy in bits as a number.
function Stats.entropy(text)
    local dist = Stats.frequency_distribution(text)

    local h = 0
    for _, p in pairs(dist) do
        if p > 0 then
            h = h - p * (math.log(p) / math.log(2))
        end
    end

    return h
end

return Stats
