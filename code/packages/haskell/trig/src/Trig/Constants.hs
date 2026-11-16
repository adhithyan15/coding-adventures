-- | Every constant of TRIG-REF-1 (PHY00a section 3.2), built from its bit pattern.
--
-- A decimal literal is only a request: "give me the double nearest this
-- decimal".  Whether the answer is the double the specification means depends on
-- the compiler.  Building each constant from the 64 bits the specification pins
-- (@castWord64ToDouble@, a reinterpretation with no arithmetic at all) removes
-- that question: the bits in the source are the bits in the machine.  The test
-- suite compares each one with the constant table of the shared fixture's
-- @manifest.json@, via 'constantTable'.
--
-- Nothing here is clever.  The comments say what each number is for, because the
-- bits alone say nothing.
module Trig.Constants
    ( -- * Circle constants
      piValue
    , twoPi
    , halfPi
      -- * Range reduction (Cody-Waite)
    , invPio2
    , pio2_1
    , pio2_2
    , pio2_3
    , pio2_4
    , pio2_5
    , magic
    , limit
    , tiny
      -- * Series coefficients
    , sinCoefficients
    , cosCoefficients
      -- * Degree conversion
    , radPerDeg
    , degPerRad
      -- * Legacy contract constants (tan, sqrt, atan)
    , tanPole
    , tanBig
    , atanTerm
    , sqrtRel
    , sqrtTiny
      -- * The whole table, for the conformance test
    , constantTable
    ) where

import Data.Word (Word64)
import GHC.Float (castWord64ToDouble)

-- | Reinterpret 64 bits as an IEEE binary64 number.  No rounding happens.
bits :: Word64 -> Double
bits = castWord64ToDouble

-- | pi, @3.141592653589793@.
piValue :: Double
piValue = bits 0x400921FB54442D18

-- | One complete rotation, @2 * pi@ (exactly: doubling only changes the exponent).
twoPi :: Double
twoPi = bits 0x401921FB54442D18

-- | A quarter rotation, @pi / 2@ (exactly: halving only changes the exponent).
halfPi :: Double
halfPi = bits 0x3FF921FB54442D18

-- | The double nearest @2 / pi@: @x * invPio2@ is roughly the number of
-- quarter-turns in @x@.
invPio2 :: Double
invPio2 = bits 0x3FE45F306DC9C883

-- | @pi / 2@ is stored as a sum of five pieces.  The first four have at most 24
-- significant bits each, so multiplying one of them by an integer @k@ with
-- @|k| < 2^29@ needs at most 24 + 29 = 53 bits: the product is exact.  That is
-- the Cody-Waite device (Cody and Waite, /Software Manual for the Elementary
-- Functions/, 1980).  The fifth piece is the remainder, rounded to 53 bits.
-- Together the five carry @pi / 2@ to about 149 bits.
pio2_1, pio2_2, pio2_3, pio2_4, pio2_5 :: Double
pio2_1 = bits 0x3FF921FB40000000 -- the first 24 bits of pi/2
pio2_2 = bits 0x3E74442D00000000 -- the next 24
pio2_3 = bits 0x3CF8469880000000 -- the next 24
pio2_4 = bits 0x3B78CC5160000000 -- the next 24
pio2_5 = bits 0x39F01B839A25204A -- the remainder

-- | @1.5 * 2^52@.  Adding it to @t@ (with @|t| < 2^51@) pushes the fraction bits
-- off the end of the significand, so @(t + magic) - magic@ is @t@ rounded to the
-- nearest integer, ties to even, using only an add and a subtract.
magic :: Double
magic = bits 0x4338000000000000

-- | @2^29@: the edge of the domain of 'Trig.sin', 'Trig.cos' and 'Trig.tan'.
limit :: Double
limit = bits 0x41C0000000000000

-- | @2^-27@: below this @sin x = x@ and @cos x = 1@ to the last bit.
tiny :: Double
tiny = bits 0x3E40000000000000

-- | @S[n] = (-1)^n / (2n+1)!@ for @n = 1 .. 9@, each rounded once, in order
-- (so the list starts at @-1/3!@ and ends at @-1/19!@).
sinCoefficients :: [Double]
sinCoefficients =
    map
        bits
        [ 0xBFC5555555555555 -- S1 = -1/3!
        , 0x3F81111111111111 -- S2 = +1/5!
        , 0xBF2A01A01A01A01A -- S3 = -1/7!
        , 0x3EC71DE3A556C734 -- S4 = +1/9!
        , 0xBE5AE64567F544E4 -- S5 = -1/11!
        , 0x3DE6124613A86D09 -- S6 = +1/13!
        , 0xBD6AE7F3E733B81F -- S7 = -1/15!
        , 0x3CE952C77030AD4A -- S8 = +1/17!
        , 0xBC62F49B46814157 -- S9 = -1/19!
        ]

-- | @C[n] = (-1)^n / (2n)!@ for @n = 2 .. 9@, in order (@+1/4!@ to @-1/18!@).
-- The @n = 1@ coefficient, @-1/2@, is applied as @0.5 * z@, which is exact.
cosCoefficients :: [Double]
cosCoefficients =
    map
        bits
        [ 0x3FA5555555555555 -- C2 = +1/4!
        , 0xBF56C16C16C16C17 -- C3 = -1/6!
        , 0x3EFA01A01A01A01A -- C4 = +1/8!
        , 0xBE927E4FB7789F5C -- C5 = -1/10!
        , 0x3E21EED8EFF8D898 -- C6 = +1/12!
        , 0xBDA93974A8C07C9D -- C7 = -1/14!
        , 0x3D2AE7F3E733B81F -- C8 = +1/16!
        , 0xBCA6827863B97D97 -- C9 = -1/18!
        ]

-- | @pi / 180@ and @180 / pi@, each rounded once.
radPerDeg, degPerRad :: Double
radPerDeg = bits 0x3F91DF46A2529D39
degPerRad = bits 0x404CA5DC1A63C1F8

-- | @1e-15@: 'Trig.tan' treats @|cos| < tanPole@ as a pole.
tanPole :: Double
tanPole = bits 0x3CD203AF9EE75616

-- | @1e308@: the finite value 'Trig.tan' returns at a pole (a pinned legacy
-- contract, not an approximation of the tangent).
tanBig :: Double
tanBig = bits 0x7FE1CCF385EBC8A0

-- | @1e-17@: the arctangent series stops once a term is smaller than this.
atanTerm :: Double
atanTerm = bits 0x3C670EF54646D497

-- | @1e-15@: the relative tolerance of the Newton square-root iteration.
sqrtRel :: Double
sqrtRel = bits 0x3CD203AF9EE75616

-- | @1e-300@: the absolute floor of the same tolerance.
sqrtTiny :: Double
sqrtTiny = bits 0x01A56E1FC2F8F359

-- | Every constant under the name @manifest.json@ gives it.
constantTable :: [(String, Double)]
constantTable =
    [ ("PI", piValue)
    , ("TWO_PI", twoPi)
    , ("HALF_PI", halfPi)
    , ("INV_PIO2", invPio2)
    , ("PIO2_1", pio2_1)
    , ("PIO2_2", pio2_2)
    , ("PIO2_3", pio2_3)
    , ("PIO2_4", pio2_4)
    , ("PIO2_5", pio2_5)
    , ("MAGIC", magic)
    , ("LIMIT", limit)
    , ("TINY", tiny)
    , ("RAD_PER_DEG", radPerDeg)
    , ("DEG_PER_RAD", degPerRad)
    , ("TAN_POLE", tanPole)
    , ("TAN_BIG", tanBig)
    , ("ATAN_TERM", atanTerm)
    , ("SQRT_REL", sqrtRel)
    , ("SQRT_TINY", sqrtTiny)
    ]
        ++ [("S" ++ show n, c) | (n, c) <- zip [1 :: Int ..] sinCoefficients]
        ++ [("C" ++ show n, c) | (n, c) <- zip [2 :: Int ..] cosCoefficients]
