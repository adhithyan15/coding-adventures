-- | Polynomial — coefficient-array polynomial arithmetic over GF(256).
--
-- A polynomial is represented as a coefficient array where __index equals degree__:
--
-- @
--   [3, 0, 2]  →  3 + 0·x + 2·x²  =  3 + 2x²
--   [1, 2, 3]  →  1 + 2x + 3x²
--   []         →  the zero polynomial
-- @
--
-- This __little-endian__ (low-degree-first) representation makes addition
-- trivially position-aligned and keeps Horner's method natural to read.
--
-- == Normalization
--
-- All operations return __normalized__ polynomials — trailing zeros
-- (high-degree zero coefficients) are stripped:
--
-- @
--   normalize [1, 0, 0] = [1]   -- constant polynomial 1
--   normalize [0]       = []    -- zero polynomial
-- @
--
-- The zero polynomial is @[]@. Its degree is @-1@ by convention; this
-- sentinel makes polynomial long division terminate cleanly.
--
-- == Arithmetic over GF(256)
--
-- All coefficient arithmetic uses GF(256) operations from the @GF256@ module:
--
-- * Coefficient addition\/subtraction = XOR ('gfAdd' \/ 'gfSub')
-- * Coefficient multiplication = 'gfMul' (log\/antilog tables)
-- * Coefficient division = 'gfDiv'
--
-- This is the polynomial layer used by Reed-Solomon encoding and decoding.
--
-- == Applications
--
-- * Reed-Solomon generator polynomial construction
-- * RS encoding (polynomial long division)
-- * RS decoding (syndrome evaluation, Berlekamp-Massey, Forney)
module Polynomial
  ( -- * Types
    Poly (..)

    -- * Fundamentals
  , polyNormalize
  , polyDegree
  , polyZero
  , polyOne

    -- * Arithmetic
  , polyAdd
  , polySub
  , polyMul
  , polyDivMod
  , polyDiv
  , polyMod

    -- * Evaluation
  , polyEval

    -- * GCD
  , polyGcd

    -- * Helpers
  , polyScale
  ) where

import GF256 (GF256, gfAdd, gfSub, gfMul, gfDiv, gfZero, gfOne)

-- ---------------------------------------------------------------------------
-- Types
-- ---------------------------------------------------------------------------

-- | A polynomial over GF(256), stored as a coefficient list in __little-endian
-- order__ (index = degree).
--
-- Invariant: the last element of 'coeffs' is non-zero (normalized form),
-- except for the zero polynomial which is represented as @Poly []@.
newtype Poly = Poly { coeffs :: [GF256] }
    deriving (Eq)

instance Show Poly where
    show (Poly []) = "Poly []"
    show (Poly cs) = "Poly " ++ show cs

-- ---------------------------------------------------------------------------
-- Normalization
-- ---------------------------------------------------------------------------

-- | Remove trailing zeros from a polynomial.
--
-- Trailing zeros are zero-coefficient high-degree terms. They do not change
-- the polynomial's value but can confuse degree comparisons and the long
-- division loop.
--
-- @
--   polyNormalize (Poly [1, 0, 0]) = Poly [1]
--   polyNormalize (Poly [0])       = Poly []
--   polyNormalize (Poly [1, 2, 3]) = Poly [1, 2, 3]
-- @
polyNormalize :: Poly -> Poly
polyNormalize (Poly cs) = Poly (dropTrailingZeros cs)
  where
    dropTrailingZeros [] = []
    dropTrailingZeros xs =
        let xs' = reverse (dropWhile (== gfZero) (reverse xs))
        in  xs'

-- ---------------------------------------------------------------------------
-- Degree and Identity Polynomials
-- ---------------------------------------------------------------------------

-- | Return the degree of a polynomial.
--
-- The degree is the index of the highest non-zero coefficient.
-- By convention the zero polynomial has degree @-1@:
--
-- @
--   polyDegree (Poly [3, 0, 2]) = 2
--   polyDegree (Poly [7])       = 0
--   polyDegree (Poly [])        = -1
--   polyDegree (Poly [0, 0])    = -1  (normalizes to [])
-- @
--
-- The sentinel value @-1@ for the zero polynomial lets the polynomial long
-- division loop terminate cleanly.
polyDegree :: Poly -> Int
polyDegree p =
    let Poly cs = polyNormalize p
    in  length cs - 1

-- | The zero polynomial (additive identity).
polyZero :: Poly
polyZero = Poly []

-- | The constant polynomial 1 (multiplicative identity).
polyOne :: Poly
polyOne = Poly [gfOne]

-- ---------------------------------------------------------------------------
-- Addition and Subtraction
-- ---------------------------------------------------------------------------

-- | Add two polynomials term-by-term over GF(256).
--
-- Addition is coefficient-wise XOR (since addition in GF(256) = XOR).
-- The shorter polynomial is implicitly extended with zeros.
--
-- @
--   polyAdd (Poly [1, 2, 3]) (Poly [4, 5]) = Poly [5, 7, 3]
--   --  (1 XOR 4, 2 XOR 5, 3 XOR 0) = (5, 7, 3)
-- @
polyAdd :: Poly -> Poly -> Poly
polyAdd (Poly as) (Poly bs) = polyNormalize (Poly result)
  where
    len    = max (length as) (length bs)
    pad xs = xs ++ replicate (len - length xs) gfZero
    result = zipWith gfAdd (pad as) (pad bs)

-- | Subtract polynomial @b@ from @a@ over GF(256).
--
-- In GF(256), subtraction equals addition (XOR), so this is identical to
-- 'polyAdd'. Provided for semantic clarity in divmod and Reed-Solomon.
polySub :: Poly -> Poly -> Poly
polySub (Poly as) (Poly bs) = polyNormalize (Poly result)
  where
    len    = max (length as) (length bs)
    pad xs = xs ++ replicate (len - length xs) gfZero
    result = zipWith gfSub (pad as) (pad bs)

-- | Multiply a polynomial by a GF(256) scalar.
--
-- Each coefficient is multiplied by the scalar using GF(256) multiplication.
-- Used internally by long division and Reed-Solomon.
polyScale :: GF256 -> Poly -> Poly
polyScale s (Poly cs) = polyNormalize (Poly (map (gfMul s) cs))

-- ---------------------------------------------------------------------------
-- Multiplication
-- ---------------------------------------------------------------------------

-- | Multiply two polynomials using polynomial convolution over GF(256).
--
-- If @a@ has degree @m@ and @b@ has degree @n@, the result has degree @m+n@.
--
-- Algorithm: for each pair @(i, j)@, add @a[i] × b[j]@ to @result[i+j]@.
-- Coefficient sums use GF(256) addition (XOR).
--
-- @
--   -- (1 + 2x) × (3 + 4x) = 3 + 10x + 8x²
--   polyMul (Poly [1, 2]) (Poly [3, 4]) = Poly [3, 10, 8]
-- @
--
-- Note: in GF(256), @10 = 1 XOR 2 XOR 4 XOR ... wait... 1·4 + 2·3@.
-- Actually 1·4 = 4 (GF), 2·3 = gfMul 2 3 = 6, and 4 XOR 6 = 2... hmm.
-- Actually in __real-number__ polynomial arithmetic 1·4 + 2·3 = 10, but
-- over GF(256) these are GF(256) multiplications:
-- @gfMul 1 4 = 4, gfMul 2 3 = 6, gfAdd 4 6 = 2@.
-- Over GF(256): @(1+2x)(3+4x) = 3 XOR (gfMul 1 4 XOR gfMul 2 3)x XOR gfMul 2 4 · x²@.
--
-- The example above uses __integer__ coefficients for clarity; real RS use
-- uses GF(256) coefficients throughout.
polyMul :: Poly -> Poly -> Poly
polyMul (Poly []) _ = polyZero
polyMul _ (Poly []) = polyZero
polyMul (Poly as) (Poly bs) = polyNormalize (Poly result)
  where
    lenA   = length as
    lenB   = length bs
    result = foldr combine (replicate (lenA + lenB - 1) gfZero) indexed
    indexed = [ (i, j, gfMul a b)
              | (i, a) <- zip [0..] as
              , (j, b) <- zip [0..] bs ]
    combine (i, j, v) xs =
        let k = i + j
        in  take k xs ++ [gfAdd (xs !! k) v] ++ drop (k + 1) xs

-- ---------------------------------------------------------------------------
-- Division
-- ---------------------------------------------------------------------------

-- | Polynomial long division over GF(256), returning @(quotient, remainder)@.
--
-- Given @a@ and @b@ (b ≠ zero), finds @q@ and @r@ such that:
-- @a = b × q + r@ and @polyDegree r < polyDegree b@.
--
-- Algorithm (polynomial analog of school long division):
--
-- 1. Find the leading term of the remainder.
-- 2. Divide it by the leading term of @b@ to get the next quotient term.
-- 3. Subtract @(quotient term) × b@ from the remainder.
-- 4. Repeat until @degree(remainder) < degree(b)@.
--
-- All coefficient operations are in GF(256) (add = XOR, mul = table lookup,
-- div = table lookup).
--
-- Throws a runtime error if @b@ is the zero polynomial.
polyDivMod :: Poly -> Poly -> (Poly, Poly)
polyDivMod a b
    | polyDegree nb == -1 = error "Polynomial.polyDivMod: division by zero"
    | polyDegree na < polyDegree nb = (polyZero, na)
    | otherwise = go (coeffs na) (replicate qLen gfZero)
  where
    na   = polyNormalize a
    nb   = polyNormalize b
    degA = polyDegree na
    degB = polyDegree nb
    qLen = degA - degB + 1
    bcs  = coeffs nb

    -- Leading coefficient of the divisor (used for each division step).
    leadB = last bcs

    -- Iterative long division: r is the working remainder (low→high order),
    -- q accumulates quotient coefficients.
    go :: [GF256] -> [GF256] -> (Poly, Poly)
    go r q =
        let degR = length (dropTrailingZeros r) - 1
        in  if degR < degB
              then (polyNormalize (Poly q), polyNormalize (Poly r))
              else
                let leadR  = r !! degR
                    coeff  = gfDiv leadR leadB
                    pw     = degR - degB
                    -- Update quotient at index `pw`
                    q'     = setAt pw coeff q
                    -- Subtract coeff * b shifted by `pw`
                    r'     = subtractShifted coeff pw r
                in  go r' q'

    -- Subtract (scalar * b) shifted left by `shift` positions from xs.
    subtractShifted :: GF256 -> Int -> [GF256] -> [GF256]
    subtractShifted scalar shift xs =
        let scaled  = map (gfMul scalar) bcs
            shifted = replicate shift gfZero ++ scaled
            padLen  = max (length xs) (length shifted)
            padXs   = xs   ++ replicate (padLen - length xs) gfZero
            padSh   = shifted ++ replicate (padLen - length shifted) gfZero
        in  zipWith gfSub padXs padSh

    setAt :: Int -> a -> [a] -> [a]
    setAt i v xs = take i xs ++ [v] ++ drop (i + 1) xs

    dropTrailingZeros :: [GF256] -> [GF256]
    dropTrailingZeros = reverse . dropWhile (== gfZero) . reverse

-- | Return the quotient of 'polyDivMod'.
--
-- Throws a runtime error if @b@ is zero.
polyDiv :: Poly -> Poly -> Poly
polyDiv a b = fst (polyDivMod a b)

-- | Return the remainder of 'polyDivMod'.
--
-- This is the polynomial \"modulo\" operation: the remainder when @a@ is
-- divided by @b@. Used in Reed-Solomon encoding (remainder of message·x^n
-- divided by the generator polynomial gives the check bytes).
--
-- Throws a runtime error if @b@ is zero.
polyMod :: Poly -> Poly -> Poly
polyMod a b = snd (polyDivMod a b)

-- ---------------------------------------------------------------------------
-- Evaluation
-- ---------------------------------------------------------------------------

-- | Evaluate a polynomial at a GF(256) point using Horner's method.
--
-- Horner's method rewrites the polynomial in nested form:
--
-- @
--   a₀ + x(a₁ + x(a₂ + ... + x·aₙ))
-- @
--
-- Requires only @n@ GF(256) multiplications and @n@ additions — no powers.
--
-- Algorithm (reading from high degree to low):
--
-- @
--   acc = 0
--   for i from n downto 0:  acc = acc * x + p[i]
--   return acc
-- @
--
-- Example: evaluate @[3, 1, 2]@ (= @3 + x + 2x²@) at @x = 4@:
--
-- @
--   acc = 0
--   i=2: acc = 0*4 + 2 = 2
--   i=1: acc = 2*4 + 1 = 9   (GF arithmetic)
--   i=0: acc = 9*4 + 3 = 39  (GF arithmetic)
-- @
polyEval :: Poly -> GF256 -> GF256
polyEval p x =
    let Poly cs = polyNormalize p
    in  case cs of
          [] -> gfZero
          _  -> foldr step gfZero cs
  where
    -- Horner: fold from the high end.
    -- foldr f z [a0, a1, a2] = f a0 (f a1 (f a2 z))
    -- We want: a2*x^2 + a1*x + a0 evaluated at x
    -- Which is: a0 + x*(a1 + x*a2) = a0 + x*(a1 + x*(a2 + x*0))
    -- foldr (\ai acc -> ai `gfAdd` gfMul x acc) 0 [a0,a1,a2]
    step ai acc = gfAdd ai (gfMul x acc)

-- ---------------------------------------------------------------------------
-- GCD
-- ---------------------------------------------------------------------------

-- | Compute the GCD of two polynomials over GF(256).
--
-- Uses the Euclidean algorithm: repeatedly replace @(a, b)@ with @(b, a mod b)@
-- until @b@ is the zero polynomial. The last non-zero remainder is the GCD.
--
-- @
--   gcd(a, b):
--     while b ≠ zero:
--       a, b = b, a mod b
--     return normalize(a)
-- @
--
-- Identical to the integer GCD algorithm, with polynomial mod instead of
-- integer mod.
polyGcd :: Poly -> Poly -> Poly
polyGcd a b =
    let a' = polyNormalize a
        b' = polyNormalize b
    in  go a' b'
  where
    go u v
      | polyDegree v == -1 = polyNormalize u
      | otherwise          = go v (polyMod u v)
