module Arithmetic.Subtraction
    ( subtractorN
    ) where

import LogicGates
import Arithmetic.Addition

-- A - B = A + (NOT B) + 1
subtractorN :: [Bit] -> [Bit] -> Either String ([Bit], Bit)
subtractorN aBits bBits = do
    if length aBits /= length bBits
    then Left "Inputs must have the same length"
    else do
        invB <- mapM notGate bBits
        (diff, borrowBar) <- rippleCarryAdder aBits invB 1
        borrow <- notGate borrowBar
        return (diff, borrow)
