module Arithmetic.Comparison
    ( isZero
    , isNegative
    ) where

import LogicGates

isZero :: [Bit] -> Either String Bit
isZero bits = do
    if null bits then return 1
    else do
        let foldOr acc bit = do
                accVal <- acc
                orGate accVal bit
        anyOne <- foldl foldOr (Right 0) bits
        notGate anyOne

isNegative :: [Bit] -> Either String Bit
isNegative bits = do
    if null bits then return 0
    else validateBit (head bits)
