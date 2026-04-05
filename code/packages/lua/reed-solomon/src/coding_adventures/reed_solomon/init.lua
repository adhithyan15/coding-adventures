-- reed_solomon — Reed-Solomon error-correcting codes over GF(256)
--
-- # What Is Reed-Solomon?
--
-- Reed-Solomon (RS) is a block error-correcting code invented by Irving Reed
-- and Gustave Solomon in 1960. The core idea: add redundancy bytes computed
-- from the data so that even if some bytes are corrupted in transit, the
-- original can be recovered.
--
-- Real-world uses:
--
--   QR codes     — Up to 30% of the symbol can be scratched and still decoded.
--   CDs / DVDs   — CIRC two-level RS corrects scratches and burst errors.
--   Hard drives  — Firmware corrects sector-level errors before the OS sees them.
--   Voyager 1    — Images sent across 20+ billion km of lossy radio.
--   RAID-6       — The two parity drives ARE an (n, n-2) RS code over GF(256).
--
-- # How It Fits in the MA Series
--
--   MA00 polynomial   — coefficient-array polynomial arithmetic (conceptual base)
--   MA01 gf256        — GF(2^8) field arithmetic (add=XOR, mul=table lookup)
--   MA02 reed-solomon — RS encoding and decoding (THIS PACKAGE)
--   MA03 qr-encoder   — QR code generation; calls MA02 for check codewords
--
-- An RS encoder is just polynomial multiplication over GF(256).
-- An RS decoder is Berlekamp-Massey + Chien search + Forney: polynomial
-- operations over GF(256) composed into a five-step pipeline.
--
-- # Code Parameters
--
-- An RS code is described by three numbers [n, k, d]:
--
--   n        = total block length (message + check bytes)
--   k        = message length (data bytes)
--   n - k    = n_check = number of check bytes (the API parameter)
--   t        = floor(n_check / 2) = maximum errors correctable
--   d        = n - k + 1 = minimum Hamming distance between valid codewords
--
-- For GF(256): n ≤ 255 (the field has 256 elements; 0 is special).
--
-- Capability table:
--
--   n_check | t (errors correctable) | overhead
--   --------+------------------------+---------
--      2    |          1             | 2 bytes per k bytes
--      4    |          2             | 4 bytes per k bytes
--      8    |          4             | 8 bytes per k bytes
--     16    |          8             | 16 bytes per k bytes
--     32    |         16             | 32 bytes per k bytes
--
-- # Polynomial Conventions (CRITICAL)
--
-- There are TWO polynomial representations in this code:
--
-- 1. Codewords (the byte arrays seen by encode/decode):
--    BIG-ENDIAN: index 1 = highest-degree coefficient
--    codeword = [c_{n-1}, c_{n-2}, ..., c_1, c_0]
--    This is the standard RS/QR convention.
--
-- 2. Internal polynomials (generator g, error locator Λ, error evaluator Ω):
--    LITTLE-ENDIAN: index 1 = constant term (degree 0)
--    poly = [p_0, p_1, p_2, ...]  where  p(x) = p_0 + p_1·x + p_2·x² + ...
--
-- This convention boundary is explicitly marked in every function below.
--
-- # Quick Start
--
--   local rs = require("coding_adventures.reed_solomon")
--
--   -- Encode 4 bytes with 2 check bytes (can correct 1 error)
--   local codeword = rs.encode({4, 3, 2, 1}, 2)
--   -- codeword is a 6-byte table: {4, 3, 2, 1, check0, check1}
--
--   -- Corrupt a byte and decode
--   codeword[2] = 0xFF
--   local recovered = rs.decode(codeword, 2)
--   -- recovered = {4, 3, 2, 1}

local gf = require("coding_adventures.gf256")

local M = {}

M.VERSION = "0.1.0"

-- ============================================================================
-- poly_eval_be(p, x) — Evaluate a big-endian GF(256) polynomial at x
-- ============================================================================
--
-- Big-endian: p[1] is the highest-degree coefficient.
-- Horner's method: acc = (...((p[1]·x + p[2])·x + p[3])·x ... + p[n])
--
-- Step by step for p = [a, b, c] (degree 2):
--   acc = 0
--   acc = gf.add(gf.multiply(0, x), a)  = a
--   acc = gf.add(gf.multiply(a, x), b)  = a·x + b
--   acc = gf.add(gf.multiply(a·x+b, x), c) = a·x² + b·x + c  ✓
--
-- Used for syndrome evaluation: S_j = codeword(α^j).
--
local function poly_eval_be(p, x)
    local acc = 0
    for i = 1, #p do
        acc = gf.add(gf.multiply(acc, x), p[i])
    end
    return acc
end

-- ============================================================================
-- poly_eval_le(p, x) — Evaluate a little-endian GF(256) polynomial at x
-- ============================================================================
--
-- Little-endian: p[1] is the constant term (coefficient of x^0).
-- We iterate from the highest degree down to 0 using Horner's method:
--
--   acc = 0
--   for i from #p down to 1:
--       acc = acc·x + p[i]
--
-- Example: p = [8, 6, 1] (LE: p(x) = 8 + 6x + x²)
--   Evaluate at x=2: 8 + 6·2 + 1·4 = 8 XOR 12 XOR 4 = 0 (root check) ✓
--
-- Used for evaluating Λ(x), Ω(x), Λ'(x) in Chien and Forney steps.
--
local function poly_eval_le(p, x)
    local acc = 0
    for i = #p, 1, -1 do
        acc = gf.add(gf.multiply(acc, x), p[i])
    end
    return acc
end

-- ============================================================================
-- poly_mul_le(a, b) — Multiply two little-endian GF(256) polynomials
-- ============================================================================
--
-- Standard polynomial (convolution) multiply. Result length = #a + #b - 1.
--
-- Schoolbook algorithm:
--   result[i+j-1] ^= a[i] · b[j]    for all 1 ≤ i ≤ #a, 1 ≤ j ≤ #b
--
-- In GF(256), addition is XOR, so ^= is the right accumulation operator.
--
-- Example: a = [2, 1] (= x + 2), b = [4, 1] (= x + 4)
--   result[1] = mul(2, 4) = 8
--   result[2] = mul(2, 1) XOR mul(1, 4) = 2 XOR 4 = 6
--   result[3] = mul(1, 1) = 1
--   → [8, 6, 1]  = x² + 6x + 8  ✓
--
-- Used in: build_generator, and Forney's Ω = S·Λ computation.
--
local function poly_mul_le(a, b)
    if #a == 0 or #b == 0 then return {} end
    local result = {}
    for i = 1, #a + #b - 1 do result[i] = 0 end
    for i = 1, #a do
        for j = 1, #b do
            result[i + j - 1] = gf.add(result[i + j - 1], gf.multiply(a[i], b[j]))
        end
    end
    return result
end

-- ============================================================================
-- poly_mod_be(dividend, gen_be) — Big-endian polynomial long division remainder
-- ============================================================================
--
-- Both dividend and gen_be are big-endian (index 1 = highest degree).
-- The divisor (gen_be) MUST be monic (gen_be[1] = 1). This is guaranteed
-- because our generator polynomial is monic by construction.
--
-- Algorithm — schoolbook synthetic division:
--
--   rem = copy(dividend)
--   for i = 1 to #dividend - #gen_be + 1:
--       lead = rem[i]
--       if lead == 0: continue
--       for j = 1 to #gen_be:
--           rem[i + j - 1] XOR= gf.multiply(lead, gen_be[j])
--
-- After the loop, the last (#gen_be - 1) elements of rem are the remainder.
--
-- Why it works: at each step, we zero out rem[i] by subtracting lead·g(x)·x^k
-- (where k positions rem[i] as the leading term). In characteristic 2,
-- subtraction = addition = XOR, so XOR-ing does both.
--
-- Returns a table of exactly (#gen_be - 1) elements = n_check bytes.
--
local function poly_mod_be(dividend, gen_be)
    local rem = {}
    for i = 1, #dividend do rem[i] = dividend[i] end

    local div_len = #gen_be
    if #rem < div_len then return rem end

    local steps = #rem - div_len + 1
    for i = 1, steps do
        local lead = rem[i]
        if lead ~= 0 then
            for j = 1, div_len do
                rem[i + j - 1] = gf.add(rem[i + j - 1], gf.multiply(lead, gen_be[j]))
            end
        end
    end

    -- Return the last (div_len - 1) elements — the remainder.
    local n_check = div_len - 1
    local result = {}
    for i = 1, n_check do
        result[i] = rem[#rem - n_check + i]
    end
    return result
end

-- ============================================================================
-- inv_locator(p, n) — Inverse error locator X_p⁻¹ for position p in length-n codeword
-- ============================================================================
--
-- Big-endian convention: position p (1-indexed in Lua, but 0-indexed in the math)
-- corresponds to degree (n - 1 - p_zero_based) = (n - p) when using 1-based p.
--
-- Using 0-based position p0 (= p - 1 in Lua):
--   X_{p0}  = α^{n-1-p0}          (error locator number)
--   X_{p0}⁻¹ = α^{(p0 + 256 - n) mod 255}
--
-- The +256 before the modulo ensures non-negative exponent.
--
-- Example: last byte (p0 = n-1):
--   exp = (n-1 + 256 - n) mod 255 = 255 mod 255 = 0 → α^0 = 1
--
-- This function takes p0 (0-indexed position) and n (codeword length).
--
local function inv_locator(p0, n)
    local exp = (p0 + 256 - n) % 255
    return gf.power(2, exp)
end

-- ============================================================================
-- build_generator(n_check) — Build the RS generator polynomial
-- ============================================================================
--
-- The generator g(x) is the product of n_check linear factors:
--
--   g(x) = (x + α¹)(x + α²) ⋯ (x + α^{n_check})
--
-- where α = 2 is the primitive element of GF(256).
-- (In characteristic 2: x - α^i = x + α^i since subtraction = addition = XOR)
--
-- Return value: a LITTLE-ENDIAN coefficient table (index 1 = constant term).
-- Length = n_check + 1. The last element (highest degree) is always 1 (monic).
--
-- Algorithm: start with g = {1}. At each step i, multiply in (x + α^i):
--
--   factor = {α^i, 1}         ← [α^i, 1] means α^i + 1·x (LE)
--   g = poly_mul_le(g, factor)
--
-- Worked example for n_check = 2:
--
--   Start: g = {1}
--   i=1: α¹ = gf.power(2,1) = 2
--     factor = {2, 1}
--     g = poly_mul_le({1}, {2, 1}) = {2, 1}
--   i=2: α² = gf.power(2,2) = 4
--     factor = {4, 1}
--     g = poly_mul_le({2, 1}, {4, 1})
--       result[1] = mul(2,4) = 8
--       result[2] = mul(2,1) XOR mul(1,4) = 2 XOR 4 = 6
--       result[3] = mul(1,1) = 1
--     g = {8, 6, 1}
--
-- CROSS-LANGUAGE TEST VECTOR: build_generator(2) == {8, 6, 1}
--
-- Verify α¹=2 is a root: g(2) = 8 + 6·2 + 1·4 = 8 XOR 12 XOR 4 = 0 ✓
-- Verify α²=4 is a root: g(4) = 8 + 6·4 + 1·16 = 8 XOR 24 XOR 16 = 0 ✓
--
-- @error "InvalidInput: n_check must be a positive even number" if n_check is 0 or odd.
--
function M.build_generator(n_check)
    if n_check == 0 or n_check % 2 ~= 0 then
        error("InvalidInput: n_check must be a positive even number, got " .. n_check)
    end

    -- Start with the constant polynomial g(x) = 1.
    local g = {1}

    for i = 1, n_check do
        -- Compute α^i, the i-th root we are adding.
        local alpha_i = gf.power(2, i)

        -- The linear factor in LE form: (x + α^i) = α^i + 1·x → {alpha_i, 1}
        local factor = {alpha_i, 1}

        -- Multiply the current generator by this new factor.
        -- poly_mul_le works in LE, which is what we use for internal polys.
        g = poly_mul_le(g, factor)
    end

    return g
end

-- ============================================================================
-- syndromes(received, n_check) — Compute syndrome values for a received codeword
-- ============================================================================
--
-- S_j = received(α^j)   for j = 1, 2, ..., n_check
--
-- A valid (error-free) codeword satisfies received(α^j) = 0 for all j,
-- because the codeword polynomial is divisible by g(x) = ∏(x + α^j), which
-- means each α^j is a root.
--
-- How syndromes encode errors:
--   An error of magnitude e at position p (0-indexed) contributes
--   e · (α^j)^{n-1-p} = e · X_p^j  to S_j, where X_p = α^{n-1-p}.
--
-- If every syndrome is zero → no errors detected.
-- Any non-zero syndrome → errors present.
--
-- Parameters:
--   received  — 1-indexed Lua table of integer bytes (possibly corrupted codeword)
--   n_check   — number of check bytes in the codeword
--
-- Returns: 1-indexed table of n_check ints in [0, 255].
--
function M.syndromes(received, n_check)
    local synds = {}
    for j = 1, n_check do
        -- Evaluate the big-endian polynomial received(x) at x = α^j = gf.power(2, j).
        synds[j] = poly_eval_be(received, gf.power(2, j))
    end
    return synds
end

-- ============================================================================
-- error_locator(syndromes) — Berlekamp-Massey: find the error locator polynomial Λ
-- ============================================================================
--
-- The Berlekamp-Massey algorithm finds the shortest linear feedback shift register
-- (LFSR) that generates the given syndrome sequence.
--
-- # Why This Finds Errors
--
-- If errors occurred at positions with locator numbers X₁, X₂, ..., X_v
-- (where X_k = α^{n-1-p_k}), the error locator polynomial is:
--
--   Λ(x) = ∏_k (1 - X_k · x)   with  Λ(0) = Λ[1] = 1  (LE constant term)
--
-- The ROOTS of Λ(x) are X_k⁻¹ — the inverses of the error locator numbers.
-- Finding these roots (via Chien search) reveals where the errors are.
--
-- # Algorithm (Massey 1969)
--
-- Working in LITTLE-ENDIAN (index = degree):
--
--   C = {1}     ← current error locator Λ(x)
--   B = {1}     ← previous Λ(x) before last update
--   L = 0       ← number of errors found so far
--   x_shift = 1 ← iterations since last update
--   b_scale = 1 ← discrepancy at the last update step
--
--   for n = 1 to #syndromes:
--       -- Compute discrepancy d
--       d = S[n]
--       for j = 1 to L:
--           d = d XOR gf.multiply(C[j+1], S[n-j])
--             ↑ C[j+1] because LE: C[1]=Λ_0=1, C[2]=Λ_1, ...
--
--       if d == 0: x_shift += 1             (no update)
--       elif 2*L < n:                        (found more errors — grow Λ)
--           T = copy(C)
--           scale = d / b_scale
--           extend C and XOR in scale * x^{x_shift} * B
--           L = n - L
--           B = T, b_scale = d, x_shift = 1
--       else:                                (consistent update without growing)
--           scale = d / b_scale
--           extend C and XOR in scale * x^{x_shift} * B
--           x_shift += 1
--
-- The "extend C and XOR in" step:
--   For k = 1 to #B:
--       ensure C[x_shift + k] exists (pad with zeros if needed)
--       C[x_shift + k] XOR= gf.multiply(scale, B[k])
--
-- Returns Λ(x) in LE form, Λ[1] = 1 (constant term).
--
function M.error_locator(syndromes)
    local two_t = #syndromes

    -- Current error locator Λ (LE: C[1]=1 = constant term)
    local C = {1}
    -- Previous Λ before last update
    local B = {1}
    -- Number of errors found
    local big_L = 0
    -- Iterations since last update
    local x_shift = 1
    -- Discrepancy at last update
    local b_scale = 1

    for n = 1, two_t do

        -- ----------------------------------------------------------------
        -- Compute discrepancy d = S[n] + Σ_{j=1}^{L}  Λ[j+1] · S[n-j]
        --
        -- In LE: C[1]=Λ_0=1, C[2]=Λ_1, ..., so Λ_j = C[j+1].
        -- ----------------------------------------------------------------
        local d = syndromes[n]
        for j = 1, big_L do
            if j + 1 <= #C and n - j >= 1 then
                d = gf.add(d, gf.multiply(C[j + 1], syndromes[n - j]))
            end
        end

        -- ----------------------------------------------------------------
        -- Update rule
        -- ----------------------------------------------------------------
        if d == 0 then
            -- Syndrome step is consistent with current Λ — nothing to update.
            x_shift = x_shift + 1

        elseif 2 * big_L < n then
            -- Found more errors than currently modelled — extend Λ.
            --
            -- The update is: C := C XOR (d/b_scale · x^{x_shift} · B)
            -- In LE index terms: C[x_shift + k] XOR= scale * B[k]
            -- where k ranges from 1 to #B (1-indexed).
            local T = {}
            for i = 1, #C do T[i] = C[i] end  -- save copy of C

            local scale = gf.divide(d, b_scale)
            local target_len = x_shift + #B
            -- Extend C if needed
            for i = #C + 1, target_len do C[i] = 0 end
            for k = 1, #B do
                C[x_shift + k] = gf.add(C[x_shift + k], gf.multiply(scale, B[k]))
            end

            big_L = n - big_L
            B = T
            b_scale = d
            x_shift = 1

        else
            -- Consistent update — adjust Λ without growing the degree.
            local scale = gf.divide(d, b_scale)
            local target_len = x_shift + #B
            for i = #C + 1, target_len do C[i] = 0 end
            for k = 1, #B do
                C[x_shift + k] = gf.add(C[x_shift + k], gf.multiply(scale, B[k]))
            end
            x_shift = x_shift + 1
        end
    end

    return C
end

-- ============================================================================
-- _chien_search(lam, n) — Find all error positions by exhaustive evaluation
-- ============================================================================
--
-- A position p0 (0-indexed) is an error location if and only if:
--   Λ(X_{p0}⁻¹) = 0
-- where X_{p0}⁻¹ = α^{(p0 + 256 - n) mod 255}  (the inv_locator function).
--
-- We test all n positions p0 = 0, 1, ..., n-1 and collect the zeros.
--
-- Why exhaustive? Λ has at most t roots in the field. The field has 255 non-zero
-- elements. Since n ≤ 255, we simply check all n candidate positions.
--
-- Returns: 1-indexed table of 0-indexed error positions.
--
local function chien_search(lam, n)
    local positions = {}
    for p0 = 0, n - 1 do
        local xi_inv = inv_locator(p0, n)
        if poly_eval_le(lam, xi_inv) == 0 then
            positions[#positions + 1] = p0
        end
    end
    return positions
end

-- ============================================================================
-- _forney(lam, synds, positions, n) — Compute error magnitudes
-- ============================================================================
--
-- Once we know WHERE errors are, Forney's formula tells us BY HOW MUCH each
-- symbol was corrupted.
--
-- For each error at 0-indexed position p0 with X_{p0}⁻¹ = α^{(p0+256-n) mod 255}:
--
--   e_{p0} = Ω(X_{p0}⁻¹) / Λ'(X_{p0}⁻¹)
--
-- where:
--
-- 1. Ω(x) = (S(x) · Λ(x)) mod x^{n_check}
--    S(x) = S₁ + S₂x + S₃x² + ... + S_{n_check}x^{n_check-1}  (LE, 0-indexed)
--    We multiply S(x) · Λ(x) then keep only the first n_check terms.
--
-- 2. Λ'(x) = formal derivative of Λ in GF(2^8):
--    In characteristic 2, the derivative of ax^n is n·a·x^{n-1}.
--    But n = 2k → 2·a = 0 (since char=2), so EVEN-degree terms vanish.
--    Only ODD-degree terms of Λ survive:
--
--    If Λ = [Λ_0, Λ_1, Λ_2, Λ_3, ...] (LE, 1-indexed in Lua):
--    Λ'(x) = Λ_1 + Λ_3·x² + Λ_5·x⁴ + ...
--    In LE 1-indexed Lua table:
--      Λ'[1] = Λ[2]  (coefficient of x^0 in Λ')
--      Λ'[2] = 0     (coefficient of x^1 in Λ' = 0, no x^1 term)
--      Λ'[3] = Λ[4]  (coefficient of x^2 in Λ')
--      Λ'[4] = 0
--    i.e., odd-indexed positions of Λ' ← even-indexed positions of Λ.
--
-- 3. No X_p scaling factor (b=1 convention).
--
-- @error "TooManyErrors" if Λ'(X_p⁻¹) = 0 at any error locator.
--
local function forney(lam, synds, positions, n)
    local two_t = #synds

    -- ----------------------------------------------------------------
    -- Step 4a: Ω(x) = (S(x) · Λ(x)) mod x^{n_check}
    --
    -- S(x) in LE form: S[1]=S₁, S[2]=S₂, ..., S[n_check]=S_{n_check}
    -- (same as the synds table).
    -- ----------------------------------------------------------------
    local omega_full = poly_mul_le(synds, lam)
    -- Truncate to first n_check (= two_t) terms (degrees 0..n_check-1).
    local omega = {}
    for i = 1, two_t do
        omega[i] = omega_full[i] or 0
    end

    -- ----------------------------------------------------------------
    -- Step 4b: Formal derivative Λ'(x)
    --
    -- Λ in LE (Lua 1-indexed): Λ[1]=Λ_0, Λ[2]=Λ_1, Λ[3]=Λ_2, Λ[4]=Λ_3, ...
    --
    -- Λ'_k = d/dx coefficient:
    --   Λ'[k] = coefficient of x^{k-1} in Λ'(x)
    --
    -- From the derivative rule (only odd original-degree terms survive):
    --   Λ'(x) = Λ_1 + Λ_3·x² + Λ_5·x⁴ + ...
    --
    -- In 1-indexed Lua:
    --   The Λ_1 term: Λ[2] in Λ → becomes Λ'[1] in Λ' (coefficient of x^0)
    --   The Λ_3 term: Λ[4] in Λ → becomes Λ'[3] in Λ' (coefficient of x^2)
    --   The Λ_5 term: Λ[6] in Λ → becomes Λ'[5] in Λ' (coefficient of x^4)
    --
    -- Pattern: Λ'[2k-1] = Λ[2k]  for k = 1, 2, 3, ...
    --          Λ'[even]  = 0
    -- ----------------------------------------------------------------
    local lambda_prime = {}
    for i = 1, #lam - 1 do lambda_prime[i] = 0 end

    -- Iterate over even-indexed positions of Λ (= Λ_1, Λ_3, ... original terms)
    for j = 2, #lam, 2 do
        -- Λ[j] is the coefficient of x^{j-1} in Λ(x).
        -- j-1 is ODD (since j is even), so this term survives differentiation.
        -- After differentiation, it becomes coefficient of x^{j-2} in Λ'(x).
        -- In 1-indexed LE: position (j-2)+1 = j-1 in lambda_prime.
        lambda_prime[j - 1] = gf.add(lambda_prime[j - 1] or 0, lam[j])
    end

    -- ----------------------------------------------------------------
    -- Step 4c: Compute magnitudes using Forney's formula
    -- ----------------------------------------------------------------
    local magnitudes = {}
    for idx = 1, #positions do
        local p0 = positions[idx]
        local xi_inv = inv_locator(p0, n)
        local omega_val = poly_eval_le(omega, xi_inv)
        local lp_val    = poly_eval_le(lambda_prime, xi_inv)
        if lp_val == 0 then
            error("TooManyErrors: formal derivative evaluated to zero at error position " .. p0)
        end
        magnitudes[idx] = gf.divide(omega_val, lp_val)
    end

    return magnitudes
end

-- ============================================================================
-- encode(message, n_check) — Systematic RS encoding
-- ============================================================================
--
-- Systematic means the original message bytes appear UNCHANGED at the start of
-- the output, followed by the computed check bytes:
--
--   codeword = [ message bytes (k) | check bytes (n_check) ]
--
-- Algorithm:
--
-- 1. Build generator polynomial g (LE).
-- 2. Reverse g to BE (g_be[1]=1 monic, highest degree first).
-- 3. Form shifted = message || {0, 0, ..., 0}  (n_check zeros appended)
--    This represents M(x)·x^{n_check} in big-endian.
-- 4. Compute remainder R = shifted mod g_be (big-endian division).
-- 5. Output: message ++ R  (padded to exactly n_check bytes if needed).
--
-- Why step 3? We want to compute M(x)·x^{n_check} mod g(x).
-- Appending zeros to a big-endian array multiplies the polynomial by x^{n_check}.
--
-- Why this produces a valid codeword:
--   C(x) = M(x)·x^{n_check} XOR R(x) = Q(x)·g(x)   (by construction)
--   So C(α^i) = Q(α^i)·g(α^i) = 0  for i = 1..n_check  ✓
--
-- Parameters:
--   message  — 1-indexed Lua table of integer bytes 0-255
--   n_check  — number of check bytes (must be even, ≥ 2)
--
-- Returns: 1-indexed table of #message + n_check bytes.
--
-- @error "InvalidInput: n_check must be a positive even number"
-- @error "InvalidInput: total codeword length exceeds 255"
--
function M.encode(message, n_check)
    if n_check == 0 or n_check % 2 ~= 0 then
        error("InvalidInput: n_check must be a positive even number, got " .. n_check)
    end
    local n = #message + n_check
    if n > 255 then
        error("InvalidInput: total codeword length " .. n .. " exceeds GF(256) block size limit of 255")
    end

    -- Build generator in LE, then reverse to BE for the division step.
    local g_le = M.build_generator(n_check)
    local g_be = {}
    for i = 1, #g_le do
        g_be[i] = g_le[#g_le + 1 - i]
    end
    -- g_be[1] = 1 (monic — the highest degree coefficient of g)

    -- Shifted message: message || {0,...,0}  (n_check zeros)
    -- This is M(x)·x^{n_check} in big-endian form.
    local shifted = {}
    for i = 1, #message do shifted[i] = message[i] end
    for i = 1, n_check do shifted[#message + i] = 0 end

    -- Polynomial long division: remainder = shifted mod g_be.
    local remainder = poly_mod_be(shifted, g_be)

    -- Pad remainder to exactly n_check bytes (left-pad with zeros if shorter).
    local check = {}
    local pad = n_check - #remainder
    for i = 1, pad do check[i] = 0 end
    for i = 1, #remainder do check[pad + i] = remainder[i] end

    -- Codeword = message ++ check_bytes.
    local codeword = {}
    for i = 1, #message do codeword[i] = message[i] end
    for i = 1, n_check do codeword[#message + i] = check[i] end

    return codeword
end

-- ============================================================================
-- decode(received, n_check) — RS decoding (corrects up to t = n_check/2 errors)
-- ============================================================================
--
-- Five-step pipeline:
--
--   Step 1: Compute syndromes S₁..S_{n_check}
--           All zero → no errors, return message bytes immediately.
--
--   Step 2: Berlekamp-Massey → error locator Λ(x) and error count L
--           L > t → TooManyErrors
--
--   Step 3: Chien search → error positions {p₁, ..., p_L}
--           |positions| ≠ L → TooManyErrors (Λ has wrong number of roots)
--
--   Step 4: Forney algorithm → error magnitudes {e₁, ..., e_L}
--
--   Step 5: Correct: received[p_k + 1] XOR= e_k for each k
--           (p_k is 0-indexed; Lua is 1-indexed so +1)
--
--   Return first k = #received - n_check bytes.
--
-- @error "InvalidInput: n_check must be a positive even number"
-- @error "InvalidInput: received is shorter than n_check"
-- @error "TooManyErrors: ..." if more than t errors are present
--
function M.decode(received, n_check)
    if n_check == 0 or n_check % 2 ~= 0 then
        error("InvalidInput: n_check must be a positive even number, got " .. n_check)
    end
    if #received < n_check then
        error("InvalidInput: received length " .. #received .. " < n_check " .. n_check)
    end

    local t = n_check // 2
    local n = #received
    local k = n - n_check

    -- ------------------------------------------------------------------
    -- Step 1: Syndromes
    -- ------------------------------------------------------------------
    local synds = M.syndromes(received, n_check)
    local all_zero = true
    for i = 1, n_check do
        if synds[i] ~= 0 then
            all_zero = false
            break
        end
    end
    if all_zero then
        -- No errors — return message bytes directly.
        local msg = {}
        for i = 1, k do msg[i] = received[i] end
        return msg
    end

    -- ------------------------------------------------------------------
    -- Step 2: Berlekamp-Massey → error locator Λ(x)
    -- ------------------------------------------------------------------
    local lam = M.error_locator(synds)
    -- Degree of Λ = #lam - 1 = number of errors found.
    local num_errors = #lam - 1
    if num_errors > t then
        error("TooManyErrors: Berlekamp-Massey found " .. num_errors .. " errors, capacity is " .. t)
    end

    -- ------------------------------------------------------------------
    -- Step 3: Chien search → error positions (0-indexed)
    -- ------------------------------------------------------------------
    local positions = chien_search(lam, n)
    if #positions ~= num_errors then
        error("TooManyErrors: Chien search found " .. #positions .. " roots but expected " .. num_errors)
    end

    -- ------------------------------------------------------------------
    -- Step 4: Forney algorithm → error magnitudes
    -- ------------------------------------------------------------------
    local magnitudes = forney(lam, synds, positions, n)

    -- ------------------------------------------------------------------
    -- Step 5: Apply corrections
    -- XOR each position with its magnitude.
    -- positions[i] is 0-indexed; Lua tables are 1-indexed → +1.
    -- ------------------------------------------------------------------
    local corrected = {}
    for i = 1, n do corrected[i] = received[i] end
    for i = 1, #positions do
        local p0 = positions[i]
        corrected[p0 + 1] = gf.add(corrected[p0 + 1], magnitudes[i])
    end

    -- Return only the message bytes (strip check bytes).
    local message = {}
    for i = 1, k do message[i] = corrected[i] end
    return message
end

return M
