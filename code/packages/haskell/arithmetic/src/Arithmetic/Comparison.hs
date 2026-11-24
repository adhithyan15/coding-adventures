module Arithmetic.Comparison
    ( isZero
    , isNegative
    ) where

import LogicGates

isZero :: [Bit] -> Either LogicGateError Bit
isZero bits = do
    if null bits then return 1
    else do
        let foldOr acc bit = do
                accVal <- acc
                orGate accVal bit
        anyOne <- foldl foldOr (Right 0) bits
        notGate anyOne

-- | Sign test: the most significant bit of a non-empty bit vector.  Every
-- bit is validated (not only the MSB) and an empty vector is a shape
-- error; no @head@ is used.
isNegative :: [Bit] -> Either LogicGateError Bit
isNegative bits = do
    _ <- validateBits "bits" bits
    case bits of
        (b:_) -> Right b
        []    -> Left (InvalidShape "bits must not be empty")
