-- | ReedSolomon — Reed-Solomon error-correcting codes over GF(256).
--
-- Reed-Solomon is a block error-correcting code: given a message of @k@
-- bytes, the encoder adds @nCheck@ redundancy bytes such that a decoder
-- can recover the original message even if up to @t = nCheck \/ 2@ of the
-- @k + nCheck@ bytes are corrupted.
--
-- == Where RS is Used
--
-- * __QR codes__ — up to 30% of a QR symbol can be damaged and still decode.
-- * __CDs\/DVDs__ — two-level CIRC corrects scratches.
-- * __Hard drives__ — sector-level error correction firmware.
-- * __Voyager probes__ — images transmitted across billions of kilometres.
-- * __RAID-6__ — two parity drives are exactly an @(n, n-2)@ RS code.
--
-- == Building Blocks
--
-- @
--   MA00  polynomial   — GF(256) polynomial arithmetic
--   MA01  gf256        — GF(2^8) field arithmetic (add=XOR, mul=table)
--   MA02  reed-solomon — THIS MODULE
-- @
--
-- == Systematic Encoding
--
-- The output is __systematic__: the original message bytes appear unchanged
-- at the front of the codeword, followed by the computed check bytes:
--
-- @
--   output = [ message[0] ... message[k-1] | check[0] ... check[nCheck-1] ]
-- @
--
-- == Polynomial Convention
--
-- Codeword bytes are treated as a __big-endian__ polynomial:
--
-- @
--   codeword[0]·x^{n-1} + codeword[1]·x^{n-2} + ... + codeword[n-1]
-- @
--
-- Syndrome evaluation uses this big-endian (highest-degree-first) convention.
-- The generator polynomial and Berlekamp-Massey state use little-endian
-- (index = degree) internally.
module ReedSolomon
  ( -- * Errors
    RSError (..)

    -- * Encoding
  , buildGenerator
  , encode

    -- * Decoding
  , decode
  , syndromes
  , errorLocator
  ) where

import Data.List (foldl')
import GF256     (GF256, gfAdd, gfSub, gfMul, gfDiv, gfPow, gfInv, gfZero, gfOne)

-- ---------------------------------------------------------------------------
-- Error Types
-- ---------------------------------------------------------------------------

-- | Errors that can occur during RS encoding or decoding.
data RSError
    = TooManyErrors
      -- ^ The codeword has more than @t = nCheck\/2@ corrupted bytes.
      --   The data is unrecoverable.
    | InvalidInput String
      -- ^ The input parameters are invalid (e.g. @nCheck@ is odd or 0,
      --   or the total codeword length exceeds 255).
    deriving (Show, Eq)

-- ---------------------------------------------------------------------------
-- Internal Polynomial Helpers
-- ---------------------------------------------------------------------------

-- | Evaluate a __big-endian__ GF(256) polynomial at @x@.
--
-- @p[0]@ is the highest-degree coefficient. Horner's method left-to-right:
--
-- @
--   acc = 0
--   for each byte b in p (high degree first):
--     acc = acc·x XOR b   (all in GF(256))
-- @
polyEvalBE :: [GF256] -> GF256 -> GF256
polyEvalBE p x = foldl' step gfZero p
  where
    step acc b = gfAdd (gfMul acc x) b

-- | Evaluate a __little-endian__ GF(256) polynomial at @x@ (Horner).
--
-- @p[i]@ is the coefficient of @xⁱ@. Iterates from high to low degree.
polyEvalLE :: [GF256] -> GF256 -> GF256
polyEvalLE p x = foldl' step gfZero (reverse p)
  where
    step acc c = gfAdd (gfMul acc x) c

-- | Multiply two __little-endian__ GF(256) polynomials (convolution).
--
-- @result[i+j] ^= a[i] · b[j]@  (XOR-accumulate).
polyMulLE :: [GF256] -> [GF256] -> [GF256]
polyMulLE [] _ = []
polyMulLE _ [] = []
polyMulLE as bs = foldr combine (replicate resultLen gfZero) indexed
  where
    resultLen = length as + length bs - 1
    indexed   = [ (i + j, gfMul a b)
                | (i, a) <- zip [0..] as
                , (j, b) <- zip [0..] bs ]
    combine (k, v) xs = take k xs ++ [gfAdd (xs !! k) v] ++ drop (k + 1) xs

-- | Compute the remainder of __big-endian__ polynomial division in GF(256).
--
-- The divisor must be __monic__ (leading coefficient = 1).
--
-- Algorithm: for each step, eliminate the current leading term by
-- subtracting a scaled copy of the divisor:
--
-- @
--   for i = 0 .. (len(dividend) - len(divisor)):
--     coeff = dividend[i]
--     for j = 0 .. len(divisor):
--       dividend[i+j] ^= coeff · divisor[j]
--
--   remainder = last (divLen - 1) elements
-- @
polyModBE :: [GF256] -> [GF256] -> [GF256]
polyModBE dividend divisor
    | length dividend < divLen = dividend
    | otherwise                = drop (length rem' - (divLen - 1)) rem'
  where
    divLen  = length divisor
    steps   = length dividend - divLen + 1
    rem'    = foldl' elimStep (dividend ++ []) [0..steps - 1]

    elimStep :: [GF256] -> Int -> [GF256]
    elimStep buf i =
        let coeff = buf !! i
        in  if coeff == gfZero
              then buf
              else
                let updates = zip [i..] (map (gfMul coeff) divisor)
                    applyUpdate xs (k, v) = take k xs ++ [gfAdd (xs !! k) v] ++ drop (k + 1) xs
                in  foldl' applyUpdate buf updates

-- | Compute the inverse error locator for byte position @p@ in a codeword
-- of length @n@.
--
-- In big-endian convention, position @p@ has degree @n-1-p@.
-- The locator is @X_p = α^{n-1-p}@, so @X_p⁻¹ = α^{(p+256-n) mod 255}@.
invLocator :: Int -> Int -> GF256
invLocator p n =
    let e = (p + 256 - n) `mod` 255
    in  gfPow 2 e

-- ---------------------------------------------------------------------------
-- Generator Polynomial
-- ---------------------------------------------------------------------------

-- | Build the Reed-Solomon generator polynomial for @nCheck@ check bytes.
--
-- The generator is the product of @nCheck@ linear factors:
--
-- @
--   g(x) = (x + α¹)(x + α²)…(x + α^{nCheck})
-- @
--
-- where @α = 2@ is the primitive element of GF(256).
--
-- Returns a __little-endian__ coefficient array (index = degree) of length
-- @nCheck + 1@. The last element is always @1@ (monic polynomial).
--
-- Algorithm:
--
-- @
--   g = [1]
--   for i = 1 to nCheck:
--     new_g[j] = GF256.mul(αⁱ, g[j]) XOR g[j-1]
-- @
--
-- Example: @nCheck = 2@
--
-- @
--   Start: g = [1]
--   i=1: α¹ = 2;  g = [gfMul 2 1, 1] = [2, 1]
--   i=2: α² = 4;  new[0] = gfMul 4 2 = 8
--                 new[1] = gfMul 4 1 XOR 2 = 4 XOR 2 = 6
--                 new[2] = 1
--                 g = [8, 6, 1]
-- @
--
-- Returns 'Left' ('InvalidInput') if @nCheck@ is 0 or odd.
buildGenerator :: Int -> Either RSError [GF256]
buildGenerator nCheck
    | nCheck == 0 || nCheck `mod` 2 /= 0 =
        Left (InvalidInput ("nCheck must be a positive even number, got " ++ show nCheck))
    | otherwise =
        Right (foldl' multiplyFactor [gfOne] [1..nCheck])
  where
    multiplyFactor :: [GF256] -> Int -> [GF256]
    multiplyFactor g i =
        let alphaI = gfPow 2 i
            -- New polynomial has length (length g + 1).
            -- new[j] = gfMul alphaI g[j] XOR g[j-1]
            -- where g[-1] = 0 and g[length g] = 0.
            newLen  = length g + 1
            newG    = [ let curr = if j < length g then gfMul alphaI (g !! j) else gfZero
                            prev = if j > 0        then g !! (j - 1)          else gfZero
                        in  gfAdd curr prev
                      | j <- [0..newLen - 1]
                      ]
        in  newG

-- ---------------------------------------------------------------------------
-- Encoding
-- ---------------------------------------------------------------------------

-- | Encode a message with Reed-Solomon, producing a systematic codeword.
--
-- __Systematic__ means the message bytes are unchanged in the output:
--
-- @
--   output = [ message bytes | check bytes ]
-- @
--
-- Algorithm:
--
-- 1. Build generator @g@ (little-endian of length @nCheck+1@), reverse to
--    big-endian @g_BE@.
-- 2. Form @shifted = message ++ replicate nCheck 0@ (represents
--    @M(x)·x^{nCheck}@ in big-endian).
-- 3. @remainder = polyModBE shifted g_BE@
-- 4. Output @message ++ remainder@ (padded to exactly @nCheck@ bytes).
--
-- Why it works: @C(x) = M(x)·x^{nCheck} + R(x) = Q(x)·g(x)@, so
-- @C(αⁱ) = 0@ for @i = 1…nCheck@ — all roots of @g@ are roots of @C@.
--
-- Returns 'Left' on invalid input.
encode :: [GF256] -> Int -> Either RSError [GF256]
encode message nCheck
    | nCheck == 0 || nCheck `mod` 2 /= 0 =
        Left (InvalidInput ("nCheck must be a positive even number, got " ++ show nCheck))
    | n > 255 =
        Left (InvalidInput ("total codeword length " ++ show n ++ " exceeds GF(256) block size limit of 255"))
    | otherwise =
        case buildGenerator nCheck of
          Left err -> Left err
          Right gLE ->
            let gBE     = reverse gLE          -- big-endian divisor
                shifted = message ++ replicate nCheck gfZero
                rmd     = polyModBE shifted gBE
                -- Pad remainder to exactly nCheck bytes
                padded  = replicate (nCheck - length rmd) gfZero ++ rmd
            in  Right (message ++ padded)
  where
    n = length message + nCheck

-- ---------------------------------------------------------------------------
-- Decoding
-- ---------------------------------------------------------------------------

-- | Compute the @nCheck@ syndromes of a received codeword.
--
-- @S_j = received(α^j)@ for @j = 1, …, nCheck@.
--
-- If all syndromes are zero, the codeword has no errors.
-- The codeword is evaluated as a __big-endian__ polynomial.
syndromes :: [GF256] -> Int -> [GF256]
syndromes received nCheck =
    [ polyEvalBE received (gfPow 2 j) | j <- [1..nCheck] ]

-- | Berlekamp-Massey algorithm: find the error locator polynomial.
--
-- Given syndromes @S₁, …, S_{2t}@, find the shortest LFSR generating
-- the syndrome sequence. Returns @(Λ, L)@ where @Λ@ is the error locator
-- polynomial (little-endian, @Λ[0] = 1@) and @L@ is the number of errors.
--
-- Algorithm:
--
-- @
--   C = [1], B = [1], L = 0, xShift = 1, b = 1
--
--   for n = 0 to 2t-1:
--     d = S[n] XOR ∑_{j=1}^{L} C[j]·S[n-j]
--
--     if d == 0:
--       xShift++
--     elif 2L <= n:
--       T = C; C = C XOR (d\/b)·x^{xShift}·B; L = n+1-L; B = T; b = d; xShift = 1
--     else:
--       C = C XOR (d\/b)·x^{xShift}·B; xShift++
-- @
berlekampMassey :: [GF256] -> ([GF256], Int)
berlekampMassey synds = go 0 [gfOne] [gfOne] 0 1 gfOne
  where
    twoT = length synds

    go :: Int -> [GF256] -> [GF256] -> Int -> Int -> GF256 -> ([GF256], Int)
    go n c b bigL xShift bScale
        | n >= twoT = (c, bigL)
        | otherwise =
            let -- Discrepancy: d = S[n] + sum_{j=1..L} C[j] * S[n-j]
                d = foldl' (\acc j ->
                              if j < length c && n >= j
                                then gfAdd acc (gfMul (c !! j) (synds !! (n - j)))
                                else acc)
                            (synds !! n)
                            [1..bigL]

            in  if d == gfZero
                  then go (n + 1) c b bigL (xShift + 1) bScale

                  else if 2 * bigL <= n
                    then
                      let tSave  = c
                          scale  = gfDiv d bScale
                          -- c = c XOR scale * x^xShift * b
                          c'     = xorShiftScaled c xShift scale b
                          bigL'  = n + 1 - bigL
                      in  go (n + 1) c' tSave bigL' 1 d

                    else
                      let scale = gfDiv d bScale
                          c'    = xorShiftScaled c xShift scale b
                      in  go (n + 1) c' b bigL (xShift + 1) bScale

    -- XOR (scale * x^shift * poly) into acc.
    xorShiftScaled :: [GF256] -> Int -> GF256 -> [GF256] -> [GF256]
    xorShiftScaled acc shift scale poly =
        let -- Shifted poly: [0, 0, ..., 0, scale*poly[0], scale*poly[1], ...]
            scaledShifted = replicate shift gfZero ++ map (gfMul scale) poly
            -- XOR into acc, extending if needed.
            len   = max (length acc) (length scaledShifted)
            pad xs = xs ++ replicate (len - length xs) gfZero
        in  zipWith gfAdd (pad acc) (pad scaledShifted)

-- | Exposed error locator computation (Berlekamp-Massey).
--
-- Returns @Λ(x)@ in __little-endian__ form with @Λ[0] = 1@.
--
-- Useful for diagnostics and QR decoder components.
errorLocator :: [GF256] -> [GF256]
errorLocator = fst . berlekampMassey

-- | Chien Search: find which byte positions contain errors.
--
-- Position @p@ is an error location if @Λ(X_p⁻¹) = 0@, where
-- @X_p⁻¹ = α^{(p+256-n) mod 255}@ for a codeword of length @n@.
chienSearch :: [GF256] -> Int -> [Int]
chienSearch lambda n =
    [ p | p <- [0..n-1], polyEvalLE lambda (invLocator p n) == gfZero ]

-- | Forney Algorithm: compute error magnitudes from error positions.
--
-- For each error at position @p@:
--
-- @
--   e_p = Ω(X_p⁻¹) \/ Λ'(X_p⁻¹)
-- @
--
-- where:
--
-- * @Ω(x) = (S(x) · Λ(x)) mod x^{2t}@ — error evaluator polynomial
-- * @S(x) = S₁ + S₂x + … + S_{2t}x^{2t-1}@ — syndrome polynomial (LE)
-- * @Λ'(x)@ — formal derivative of @Λ@ in characteristic 2
--
-- In GF(2), the formal derivative keeps only __odd-indexed__ coefficients:
--
-- @
--   Λ'(x) = Λ₁ + Λ₃x² + Λ₅x⁴ + …
-- @
--
-- Returns 'Left' 'TooManyErrors' if any denominator evaluates to zero.
forney :: [GF256] -> [GF256] -> [Int] -> Int -> Either RSError [GF256]
forney lambda synds positions n =
    let twoT = length synds

        -- Ω = S(x) · Λ(x) mod x^{2t}: truncate to first 2t terms.
        omegaFull = polyMulLE synds lambda
        omega     = take twoT omegaFull

        -- Formal derivative of Λ in characteristic 2.
        --
        -- For Λ = [λ₀, λ₁, λ₂, λ₃, λ₄, ...]:
        --   Λ'(x) = λ₁ + 0·x + λ₃·x² + 0·x³ + λ₅·x⁴ + …
        --
        -- Rule: Λ'[k] = Λ[k+1] when (k+1) is odd, else 0.
        --   = Λ[k+1] when k is even.
        --
        -- Implementation: iterate over even k values, reading Λ[k+1].
        lp = computeLambdaPrime lambda

    in  mapM (magnitude omega lp) positions
  where
    -- Build the formal derivative polynomial.
    -- For each even k in [0, 2, 4, ...], Λ'[k] = Λ[k+1].
    -- Odd positions are 0 (coefficient of x^{odd} in characteristic 2).
    computeLambdaPrime :: [GF256] -> [GF256]
    computeLambdaPrime lam
        | null lam  = []
        | otherwise =
            let maxIdx = length lam - 2  -- last valid index for lam[k+1]
                -- Build list of length (length lam - 1): Λ'[k] for k=0..len-2
                result = [ if even k then lam !! (k + 1) else gfZero
                         | k <- [0..maxIdx] ]
            in  result

    magnitude omega lp pos =
        let xiInv    = invLocator pos n
            omegaVal = polyEvalLE omega xiInv
            lpVal    = polyEvalLE lp   xiInv
        in  if lpVal == gfZero
              then Left TooManyErrors
              else Right (gfDiv omegaVal lpVal)

-- | Decode a received Reed-Solomon codeword, correcting up to @t = nCheck\/2@
-- errors.
--
-- Pipeline:
--
-- @
--   received
--     │
--     ▼  Step 1: Syndromes S₁…S_{nCheck}
--     │          all zero? → return message directly
--     │
--     ▼  Step 2: Berlekamp-Massey → Λ(x), error count L
--     │          L > t? → TooManyErrors
--     │
--     ▼  Step 3: Chien search → error positions {p₁…pᵥ}
--     │          |positions| ≠ L? → TooManyErrors
--     │
--     ▼  Step 4: Forney → error magnitudes {e₁…eᵥ}
--     │
--     ▼  Step 5: Correct received[pₖ] XOR= eₖ
--     │
--     ▼  Return first k = len - nCheck bytes
-- @
--
-- Returns 'Left' 'TooManyErrors' if the codeword has more than @t@ errors,
-- or 'Left' 'InvalidInput' if parameters are invalid.
decode :: [GF256] -> Int -> Either RSError [GF256]
decode received nCheck
    | nCheck == 0 || nCheck `mod` 2 /= 0 =
        Left (InvalidInput ("nCheck must be a positive even number, got " ++ show nCheck))
    | length received < nCheck =
        Left (InvalidInput ("received length " ++ show (length received)
                             ++ " < nCheck " ++ show nCheck))
    | otherwise =
        let t    = nCheck `div` 2
            nn   = length received
            k    = nn - nCheck

            -- Step 1: Syndromes
            synds = syndromes received nCheck

        in  if all (== gfZero) synds
              then Right (take k received)
              else
                let -- Step 2: Berlekamp-Massey
                    (lambda, numErrors) = berlekampMassey synds

                in  if numErrors > t
                      then Left TooManyErrors
                      else
                        let -- Step 3: Chien Search
                            positions = chienSearch lambda nn

                        in  if length positions /= numErrors
                              then Left TooManyErrors
                              else
                                -- Step 4: Forney
                                case forney lambda synds positions nn of
                                  Left err         -> Left err
                                  Right magnitudes ->
                                    -- Step 5: Apply corrections
                                    let corrected = applyCorrections received (zip positions magnitudes)
                                    in  Right (take k corrected)
  where
    -- Apply XOR corrections at error positions.
    applyCorrections :: [GF256] -> [(Int, GF256)] -> [GF256]
    applyCorrections bytes [] = bytes
    applyCorrections bytes ((pos, mag):rest) =
        let updated = take pos bytes ++ [gfAdd (bytes !! pos) mag] ++ drop (pos + 1) bytes
        in  applyCorrections updated rest
