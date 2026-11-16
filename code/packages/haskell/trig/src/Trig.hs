-- | Trigonometric functions implemented from first principles, to the one
-- reference algorithm TRIG-REF-1 (PHY00a, @code/specs/PHY00a-trig-convergence.md@).
--
-- Every @trig@ port, in every language, computes the same bits.  This module is
-- the Haskell transcription of that algorithm.  No library @sin@, @cos@, @floor@,
-- @mod'@ or @round@ is used: only IEEE binary64 @+@, @-@, @*@, comparisons and one
-- small integer mask.
--
-- == The idea in one paragraph
--
-- A sine is a repeating wave.  If we know @sin@ and @cos@ on the short stretch
-- @[-pi/4, pi/4]@, and which quarter-turn of the circle we are in, we know them
-- everywhere.  So: (1) count the whole quarter-turns @k@ in the angle, (2) subtract
-- them to leave a small remainder @r@, (3) sum a short Maclaurin series for @sin r@
-- or @cos r@, and (4) let @k mod 4@ choose which of @sin r@, @cos r@, @-sin r@,
-- @-cos r@ is the answer.
--
-- == Domain
--
-- 'sin', 'cos' and 'tan' are defined for @|x| <= 2^29@ (about 5.4e8 radians).  NaN,
-- the infinities and anything larger give NaN: a finite answer there would be
-- wrong yet look right.  Reduce a long-running phase before calling.
--
-- == Arithmetic model
--
-- One rounding per operation, no fused multiply-add, no reassociation.  GHC (9.4)
-- never fuses @a * b + c@ on its own, and it does not reorder floating-point
-- arithmetic, so the straightforward code below is the algorithm.  Each product
-- that feeds a sum is nevertheless bound to its own name, the way the
-- specification writes it, so the order of operations can be read off the source.
module Trig
    ( piValue
    , twoPi
    , halfPi
    , sin
    , cos
    , radians
    , degrees
    , sqrt
    , tan
    , atan
    , atan2
    ) where

import Data.Bits ((.&.))
import Data.List (foldl')
import GHC.Float (castWord64ToDouble)
import Prelude hiding (atan, atan2, cos, sin, sqrt, tan)
import Trig.Constants

-- | A quiet NaN.  A NaN compares false with everything, which is exactly how the
-- domain guard and the NaN rules below detect it.
nan :: Double
nan = castWord64ToDouble 0x7FF8000000000000

-- ---------------------------------------------------------------------------
-- Range reduction
-- ---------------------------------------------------------------------------

-- | Split @x@ into whole quarter-turns and a leftover: @x = k * (pi/2) + r@ with
-- @|r| <= pi/4@ (plus a few ulps, because the quotient is rounded before @k@ is
-- chosen).  Returns @(k, r)@ with @k@ an integer-valued 'Double', @|k| < 2^29@.
--
-- /Step 1, the quotient./  @t = x * (2/pi)@ is approximately the number of
-- quarter-turns.  @(t + magic) - magic@ rounds it to the nearest integer (ties to
-- even): @magic = 1.5 * 2^52@ has its units digit in the last place of the
-- significand, so the addition rounds @t@ to a whole number by the IEEE rule, and
-- the subtraction is exact.  No @floor@, no @round@, no float-to-integer cast.
--
-- /Step 2, the remainder./  @pi/2@ is irrational, so @k * (pi/2)@ cannot be formed
-- exactly.  It is stored as five pieces ('pio2_1' to 'pio2_5'), the first four so
-- short (24 bits) that @k * piece@ is exact, and removed from @x@ one at a time:
--
-- > x ----------------------------------------------------------- the angle
-- > |-- k*pio2_1 --|                                               24 bits of k*pi/2
-- >                |-- k*pio2_2 --|                                next 24
-- >                               |-- k*pio2_3 --|                 next 24
-- >                                              |- k*pio2_4 -|    next 24
-- >                                                           |k*pio2_5|  last, rounds
-- > r = what is left
--
-- The first two subtractions are exact; the last three round, and each rounding is
-- at most half an ulp of the current @r@.  Each product is bound to its own name
-- before the subtraction so there is nothing for a compiler to fuse.
reduce :: Double -> (Double, Double)
reduce x = (k, r5)
  where
    t = x * invPio2
    k = (t + magic) - magic
    m1 = k * pio2_1
    r1 = x - m1
    m2 = k * pio2_2
    r2 = r1 - m2
    m3 = k * pio2_3
    r3 = r2 - m3
    m4 = k * pio2_4
    r4 = r3 - m4
    m5 = k * pio2_5
    r5 = r4 - m5

-- | The quadrant: @k mod 4@.  @k@ is an integer-valued 'Double' with @|k| < 2^29@,
-- so 'truncate' is exact and @.&. 3@ on a two's-complement 'Int' is the floor
-- modulus (@k = -1@ gives 3, @k = -5@ gives 3).  It is the only integer operation
-- in the algorithm.
quadrant :: Double -> Int
quadrant k = truncate k .&. 3

-- ---------------------------------------------------------------------------
-- The kernels: Maclaurin series by Horner's rule on stored coefficients
-- ---------------------------------------------------------------------------

-- | @sin r@ for @|r| <= pi/4@, ten terms (@r^1 .. r^19@):
--
-- > sin r = r + r*z*(S1 + S2*z + S3*z^2 + ... + S9*z^8),     z = r*r
--
-- The coefficients are the Maclaurin series, @S[n] = (-1)^n / (2n+1)!@:
--
-- > S1 = -1/3!   S2 = +1/5!   S3 = -1/7!   S4 = +1/9!   S5 = -1/11!
-- > S6 = +1/13!  S7 = -1/15!  S8 = +1/17!  S9 = -1/19!
--
-- Horner's rule starts with the innermost (smallest) coefficient, so small terms
-- are added first, which is the accurate order.  There is no division, and one
-- multiply and one add per step cannot be grouped two ways.  The leading @r@ is
-- added last, so the dominant part of the result carries a single rounding.
sinKernel :: Double -> Double
sinKernel r = r + w'
  where
    z = r * r
    p = horner z (reverse sinCoefficients)
    w = r * z
    w' = w * p

-- | @cos r@ for @|r| <= pi/4@, ten terms (@r^0 .. r^18@):
--
-- > cos r = (1 - z/2) + z*z*(C2 + C3*z + ... + C9*z^7),      z = r*r
--
-- with @C[n] = (-1)^n / (2n)!@:
--
-- > C2 = +1/4!   C3 = -1/6!   C4 = +1/8!   C5 = -1/10!
-- > C6 = +1/12!  C7 = -1/14!  C8 = +1/16!  C9 = -1/18!
--
-- @0.5 * z@ is exact.
cosKernel :: Double -> Double
cosKernel r = u' + v'
  where
    z = r * r
    q = horner z (reverse cosCoefficients)
    u = 0.5 * z
    u' = 1.0 - u
    v = z * z
    v' = v * q

-- | Horner's rule over coefficients given highest power first: start with the
-- first, then for each further coefficient @c@ do @acc <- acc * z@, @acc <- acc + c@.
-- The multiply and the add are separate statements (two roundings), never a fused
-- multiply-add.
horner :: Double -> [Double] -> Double
horner _ [] = 0.0
horner z (c0 : cs) = foldl' step c0 cs
  where
    step acc c =
        let scaled = acc * z
         in scaled + c

-- ---------------------------------------------------------------------------
-- sin, cos, tan
-- ---------------------------------------------------------------------------

-- | Sine of an angle in radians, accurate to 3 ulp on @|x| <= 2^29@; NaN outside.
--
-- > q = k mod 4:   0 -> sin r     1 -> cos r     2 -> -sin r     3 -> -cos r
--
-- Special values: @sin(-0) = -0@ (the series would give @+0@, so the argument is
-- returned itself), and @0 < |x| < 2^-27@ returns @x@, because the next term is
-- below half an ulp.
--
-- The guard is a negated @<=@ on purpose: NaN fails every comparison, so
-- @not (|x| <= limit)@ catches NaN, both infinities and every finite value beyond
-- the domain in one test.  Rewriting it as @|x| > limit@ would let a NaN through.
sin :: Double -> Double
sin x
    | not (abs x <= limit) = nan
    | abs x < tiny = x
    | otherwise = case quadrant k of
        0 -> sinKernel r
        1 -> cosKernel r
        2 -> negate (sinKernel r)
        _ -> negate (cosKernel r)
  where
    (k, r) = reduce x

-- | Cosine of an angle in radians, accurate to 3 ulp on @|x| <= 2^29@; NaN outside.
--
-- > q = k mod 4:   0 -> cos r     1 -> -sin r     2 -> -cos r     3 -> sin r
cos :: Double -> Double
cos x
    | not (abs x <= limit) = nan
    | abs x < tiny = 1.0
    | otherwise = case quadrant k of
        0 -> cosKernel r
        1 -> negate (sinKernel r)
        2 -> negate (cosKernel r)
        _ -> sinKernel r
  where
    (k, r) = reduce x

-- | Tangent as @sin / cos@, with a pinned legacy contract at the poles: where
-- @|cos| < 1e-15@ the result is @+-1e308@ (the sign of the sine), not an
-- approximation of the tangent and not infinity.  No accuracy claim is made for
-- @tan@.  Outside the domain both @sin@ and @cos@ are NaN, the pole test is
-- false, and @NaN / NaN@ is NaN.
tan :: Double -> Double
tan angle
    | abs cosine < tanPole = if sine > 0.0 then tanBig else negate tanBig
    | otherwise = sine / cosine
  where
    sine = sin angle
    cosine = cos angle

-- ---------------------------------------------------------------------------
-- radians, degrees
-- ---------------------------------------------------------------------------

-- | Convert degrees to radians: one multiply by the bit-pinned @pi/180@ (one
-- rounding, no overflowing intermediate).  Zero keeps its sign.
radians :: Double -> Double
radians value = value * radPerDeg

-- | Convert radians to degrees: one multiply by the bit-pinned @180/pi@.
degrees :: Double -> Double
degrees value = value * degPerRad

-- ---------------------------------------------------------------------------
-- Part II: sqrt, atan, atan2 (existing algorithms, pinned to the bit)
-- ---------------------------------------------------------------------------

-- | Square root via Newton's method.  A negative argument is @Left@: the error
-- channel is the Haskell idiom and is unchanged.
sqrt :: Double -> Either String Double
sqrt value
    | value < 0.0 = Left "sqrt: input must be non-negative"
    | otherwise = Right (sqrtNonnegative value)

-- | Arctangent in the range @(-pi/2, pi/2)@.
atan :: Double -> Double
atan value
    -- atan(x) rounds exactly to x here; avoid halving subnormals and retain -0.
    | abs value <= tiny = value
    | value > 1.0 = halfPi - atanCore (1.0 / value)
    | value < -1.0 = negate halfPi - atanCore (1.0 / value)
    | otherwise = atanCore value

-- | Four-quadrant arctangent in the range @(-pi, pi]@.  A NaN in either position
-- gives NaN (the old guards fell through to @+-pi/2@ for a NaN @x@ with a nonzero
-- @y@, a finite answer to an undefined question).
atan2 :: Double -> Double -> Double
atan2 y x
    | isNaN x || isNaN y = nan
    | x > 0.0 = atan (y / x)
    | x < 0.0 && y >= 0.0 = atan (y / x) + piValue
    | x < 0.0 && y < 0.0 = atan (y / x) - piValue
    | y > 0.0 = halfPi
    | y < 0.0 = negate halfPi
    | otherwise = 0.0

sqrtNonnegative :: Double -> Double
sqrtNonnegative value
    | value == 0.0 = value
    | isInfinite value = value
    | otherwise = resultScale * iterateNewton 60 initial
  where
    (scaled, resultScale) = normalize value 1.0
    normalize current scale
        | current < 0.25 = normalize (current * 4.0) (scale * 0.5)
        | current >= 4.0 = normalize (current * 0.25) (scale * 2.0)
        | otherwise = (current, scale)
    initial = if scaled >= 1.0 then scaled else 1.0
    iterateNewton remaining guess
        | remaining == (0 :: Int) = guess
        | abs (next - guess) < sqrtRel * guess + sqrtTiny = next
        | otherwise = iterateNewton (remaining - 1) next
      where
        next = (guess + scaled / guess) / 2.0

-- | The arctangent series on @|x| <= 1@ after a half-angle reduction
-- @t = x / (1 + sqrt(1 + x*x))@, summed left to right until a term drops below
-- 1e-17 (at most 30 terms):
--
-- > term_n = ((term_(n-1) * (-t*t)) * (2n-1)) / (2n+1)
atanCore :: Double -> Double
atanCore value = 2.0 * go 1 reduced reduced
  where
    reduced = value / (1.0 + sqrtNonnegative (1.0 + value * value))
    squared = reduced * reduced
    go :: Int -> Double -> Double -> Double
    go n term total
        | n > 30 = total
        | abs next < atanTerm = total'
        | otherwise = go (n + 1) next total'
      where
        index = fromIntegral n
        next = term * negate squared * (2.0 * index - 1.0) / (2.0 * index + 1.0)
        total' = total + next
