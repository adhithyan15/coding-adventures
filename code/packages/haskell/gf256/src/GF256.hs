-- | GF256 — Galois Field GF(2^8) arithmetic.
--
-- GF(256) is the finite field with exactly 256 elements: integers 0..255.
-- Each element represents a polynomial over GF(2) of degree ≤ 7:
--
-- @
--   a₇x⁷ + a₆x⁶ + ... + a₁x + a₀,  each aᵢ ∈ {0, 1}
-- @
--
-- == Characteristic-2 Arithmetic
--
-- In GF(2), @1 + 1 = 0@. Consequences:
--
-- * Addition is XOR — no carry, no overflow.
-- * Every element is its own additive inverse: @x + x = 0@.
-- * Subtraction equals addition.
--
-- == The Primitive Polynomial
--
-- Multiplication is defined modulo the irreducible polynomial:
--
-- @
--   p(x) = x^8 + x^4 + x^3 + x^2 + 1  =  0x11D  =  285
-- @
--
-- This polynomial is __irreducible__ over GF(2) — it cannot be factored —
-- ensuring every non-zero element has a multiplicative inverse.
-- It is also __primitive__: @g = 2@ generates all 255 non-zero elements.
--
-- == Log\/Antilog Tables
--
-- Precomputed for O(1) multiply/divide:
--
-- @
--   a × b = expTable ! ((logTable ! a + logTable ! b) \`mod\` 255)
--   a \/ b = expTable ! ((logTable ! a - logTable ! b + 255) \`mod\` 255)
-- @
--
-- == Applications
--
-- * Reed-Solomon error correction (QR codes, CDs, hard drives)
-- * AES encryption (MixColumns and SubBytes)
-- * General error-correcting codes
module GF256
  ( -- * Types
    GF256

    -- * Constants
  , primitivePoly
  , gfZero
  , gfOne

    -- * Log\/Antilog Tables
  , expTable
  , logTable

    -- * Field Operations
  , gfAdd
  , gfSub
  , gfMul
  , gfDiv
  , gfPow
  , gfInv
  ) where

import Data.Array (Array, (!), listArray, accumArray)
import Data.Bits  (xor, shiftL)

-- ---------------------------------------------------------------------------
-- Types
-- ---------------------------------------------------------------------------

-- | A GF(256) element: an 'Int' in the range [0, 255].
--
-- Represents the polynomial @b₇x⁷ + ... + b₁x + b₀@ where bᵢ is bit i.
type GF256 = Int

-- ---------------------------------------------------------------------------
-- Constants
-- ---------------------------------------------------------------------------

-- | The irreducible primitive polynomial for modular reduction.
--
-- @p(x) = x^8 + x^4 + x^3 + x^2 + 1 = 0x11D = 285@
--
-- Standard for Reed-Solomon (distinct from AES which uses @0x11B@).
primitivePoly :: Int
primitivePoly = 0x11D

-- | Additive identity: 0.
gfZero :: GF256
gfZero = 0

-- | Multiplicative identity: 1.
gfOne :: GF256
gfOne = 1

-- ---------------------------------------------------------------------------
-- Log/Antilog Table Construction
-- ---------------------------------------------------------------------------
--
-- We iterate 255 times, starting at val = 1, multiplying by 2 at each step:
--
--   multiply by 2  =  shift left 1 bit
--   if result >= 256 (bit 8 set):  XOR with 0x11D  (reduce mod p(x))
--
-- This produces ALOG[0..254].  ALOG[255] = 1 (the group has order 255,
-- so g^255 = g^0 = 1; needed for inverse(1) = ALOG[255 - 0] = 1).
--
-- LOG[x] is the inverse: LOG[ALOG[i]] = i.
-- LOG[0] = 0 by convention (never accessed for valid inputs).

-- | Generate the list of 255 antilog values: [(0,1),(1,2),(2,4),...,(254,x)].
alogPairs :: [(Int, Int)]
alogPairs = zip [0..254] (take 255 (iterate step 1))
  where
    step v = let v' = v `shiftL` 1
             in  if v' >= 256 then v' `xor` primitivePoly else v'

-- | Antilogarithm (exponential) table: @expTable ! i = 2^i@ in GF(256).
--
-- Indices 0..255 are valid; @expTable ! 255 = 1@ closes the cyclic group.
--
-- Notable entries:
--
-- @
--   expTable ! 0  = 1    (g^0 = 1)
--   expTable ! 1  = 2    (g^1 = 2)
--   expTable ! 7  = 128
--   expTable ! 8  = 29   (first reduction: 256 XOR 0x11D = 29)
--   expTable ! 255 = 1   (g^255 = g^0 = 1, cyclic group)
-- @
expTable :: Array Int Int
expTable = accumArray (\_ v -> v) 0 (0, 255)
             ((255, 1) : alogPairs)

-- | Logarithm table: @logTable ! x = i@ such that @2^i = x@ in GF(256).
--
-- @logTable ! 0 = 0@ by convention (undefined; never used in arithmetic).
-- For @x ∈ 1..255@: @expTable ! (logTable ! x) = x@.
logTable :: Array Int Int
logTable = accumArray (\_ v -> v) 0 (0, 255)
             ((0, 0) : [(v, i) | (i, v) <- alogPairs])

-- ---------------------------------------------------------------------------
-- Field Operations
-- ---------------------------------------------------------------------------

-- | Add two GF(256) elements (bitwise XOR).
--
-- In characteristic 2, addition is XOR. Each bit represents a GF(2)
-- coefficient; GF(2) addition is @1+1=0 mod 2@, which is XOR.
--
-- @
--   gfAdd 0x53 0xCA = 0x99
--   gfAdd x    x   = 0     -- every element is its own additive inverse
-- @
gfAdd :: GF256 -> GF256 -> GF256
gfAdd a b = a `xor` b

-- | Subtract two GF(256) elements.
--
-- In characteristic 2, @-1 = 1@, so subtraction equals addition: XOR.
gfSub :: GF256 -> GF256 -> GF256
gfSub a b = a `xor` b

-- | Multiply two GF(256) elements using the log\/antilog tables.
--
-- @a × b = expTable ! ((logTable ! a + logTable ! b) \`mod\` 255)@
--
-- Special case: if either operand is 0, the result is 0 (zero has no log).
--
-- Time: O(1) — two table lookups and one modular addition.
--
-- Example: @gfMul 2 4 = expTable ! 3 = 8@  because @2×4 = 8@ in GF(256).
gfMul :: GF256 -> GF256 -> GF256
gfMul 0 _ = 0
gfMul _ 0 = 0
gfMul a b = expTable ! ((logTable ! a + logTable ! b) `mod` 255)

-- | Divide @a@ by @b@ in GF(256).
--
-- @a \/ b = expTable ! ((logTable ! a - logTable ! b + 255) \`mod\` 255)@
--
-- The @+255@ keeps the index non-negative when @logTable ! a < logTable ! b@.
--
-- Special case: @gfDiv 0 _ = 0@.
--
-- Throws a runtime error for @gfDiv _ 0@ (division by zero).
gfDiv :: GF256 -> GF256 -> GF256
gfDiv _ 0 = error "GF256.gfDiv: division by zero"
gfDiv 0 _ = 0
gfDiv a b = expTable ! ((logTable ! a - logTable ! b + 255) `mod` 255)

-- | Raise a GF(256) element to a non-negative integer power.
--
-- @base^exp = expTable ! ((logTable ! base * exp) \`mod\` 255)@
--
-- The multiplicative group has order 255, so @g^255 = 1@ for all non-zero @g@.
--
-- Special cases:
--
-- * @gfPow _ 0 = 1@  (any element to the 0th power is 1)
-- * @gfPow 0 _ = 0@  (0^n = 0 for n > 0)
gfPow :: GF256 -> Int -> GF256
gfPow _ 0    = 1
gfPow 0 _    = 0
gfPow base e =
    let idx = ((logTable ! base * e) `mod` 255 + 255) `mod` 255
    in  expTable ! idx

-- | Compute the multiplicative inverse of a GF(256) element.
--
-- @inverse(a) = expTable ! (255 - logTable ! a)@
--
-- Proof: @a × expTable ! (255 - logTable ! a)
--        = expTable ! (logTable ! a + 255 - logTable ! a)
--        = expTable ! 255 = 1@  ✓
--
-- Throws a runtime error for @gfInv 0@ (zero has no inverse).
gfInv :: GF256 -> GF256
gfInv 0 = error "GF256.gfInv: zero has no multiplicative inverse"
gfInv a = expTable ! (255 - logTable ! a)
