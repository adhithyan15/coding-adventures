-- | Trigonometric functions implemented from first principles.
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

import Data.Fixed (mod')
import Data.List (foldl')
import Prelude hiding (atan, atan2, cos, sin, sqrt, tan)

-- | Pi to full 'Double' precision.
piValue :: Double
piValue = 3.141592653589793

-- | One complete rotation in radians.
twoPi :: Double
twoPi = 2.0 * piValue

-- | A quarter rotation in radians.
halfPi :: Double
halfPi = piValue / 2.0

-- | Sine via a 20-term Maclaurin series after range reduction.
sin :: Double -> Double
sin angle = snd $ foldl' step (reduced, reduced) [1 .. 19]
  where
    reduced = rangeReduce angle
    step (term, total) n =
        let index = fromIntegral n
            denominator = (2.0 * index) * (2.0 * index + 1.0)
            next = term * negate (reduced * reduced) / denominator
        in (next, total + next)

-- | Cosine via a 20-term Maclaurin series after range reduction.
cos :: Double -> Double
cos angle = snd $ foldl' step (1.0, 1.0) [1 .. 19]
  where
    reduced = rangeReduce angle
    step (term, total) n =
        let index = fromIntegral n
            denominator = (2.0 * index - 1.0) * (2.0 * index)
            next = term * negate (reduced * reduced) / denominator
        in (next, total + next)

-- | Convert degrees to radians.
radians :: Double -> Double
radians value = value * piValue / 180.0

-- | Convert radians to degrees.
degrees :: Double -> Double
degrees value = value * 180.0 / piValue

-- | Square root via Newton's method.
sqrt :: Double -> Either String Double
sqrt value
    | value < 0.0 = Left "sqrt: input must be non-negative"
    | otherwise = Right (sqrtNonnegative value)

-- | Tangent as local sine divided by local cosine.
-- Near a pole, returns a large finite value with the divergence sign.
tan :: Double -> Double
tan angle
    | abs cosine < 1e-15 = if sine > 0.0 then 1e308 else -1e308
    | otherwise = sine / cosine
  where
    sine = sin angle
    cosine = cos angle

-- | Arctangent in the range @(-pi/2, pi/2)@.
atan :: Double -> Double
atan value
    | value == 0.0 = 0.0
    | value > 1.0 = halfPi - atanCore (1.0 / value)
    | value < -1.0 = negate halfPi - atanCore (1.0 / value)
    | otherwise = atanCore value

-- | Four-quadrant arctangent in the range @(-pi, pi]@.
atan2 :: Double -> Double -> Double
atan2 y x
    | x > 0.0 = atan (y / x)
    | x < 0.0 && y >= 0.0 = atan (y / x) + piValue
    | x < 0.0 && y < 0.0 = atan (y / x) - piValue
    | y > 0.0 = halfPi
    | y < 0.0 = negate halfPi
    | otherwise = 0.0

rangeReduce :: Double -> Double
rangeReduce angle
    | wrapped > piValue = wrapped - twoPi
    | otherwise = wrapped
  where
    wrapped = angle `mod'` twoPi

sqrtNonnegative :: Double -> Double
sqrtNonnegative value
    | value == 0.0 = 0.0
    | otherwise = iterateNewton 60 initial
  where
    initial = if value >= 1.0 then value else 1.0
    iterateNewton remaining guess
        | remaining == (0 :: Int) = guess
        | abs (next - guess) < 1e-15 * guess + 1e-300 = next
        | otherwise = iterateNewton (remaining - 1) next
      where
        next = (guess + value / guess) / 2.0

atanCore :: Double -> Double
atanCore value = 2.0 * snd (foldl' step (reduced, reduced) [1 .. 30])
  where
    reduced = value / (1.0 + sqrtNonnegative (1.0 + value * value))
    squared = reduced * reduced
    step (term, total) n =
        let index = fromIntegral n
            next = term * negate squared * (2.0 * index - 1.0)
                / (2.0 * index + 1.0)
        in (next, total + next)
