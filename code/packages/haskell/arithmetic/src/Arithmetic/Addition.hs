module Arithmetic.Addition
    ( halfAdder
    , fullAdder
    , rippleCarryAdder
    , incrementer
    ) where

import LogicGates

halfAdder :: Bit -> Bit -> Either String (Bit, Bit)
halfAdder a b = do
    sumBit <- xorGate a b
    carryOut <- andGate a b
    return (sumBit, carryOut)

fullAdder :: Bit -> Bit -> Bit -> Either String (Bit, Bit)
fullAdder a b carryIn = do
    (sum1, carry1) <- halfAdder a b
    (sumOut, carry2) <- halfAdder sum1 carryIn
    carryOut <- orGate carry1 carry2
    return (sumOut, carryOut)

rippleCarryAdder :: [Bit] -> [Bit] -> Bit -> Either String ([Bit], Bit)
rippleCarryAdder aBits bBits carryIn = do
    if length aBits /= length bBits
    then Left "Inputs must have the same length"
    else do
        let go cOut [] [] = return ([], cOut)
            go cOut (a:as) (b:bs) = do
                (sumOut, nextC) <- fullAdder a b cOut
                (restSum, finalC) <- go nextC as bs
                return (sumOut : restSum, finalC)
            go _ _ _ = Left "Length mismatch"
        (sumRev, finalCarry) <- go carryIn (reverse aBits) (reverse bBits)
        return (reverse sumRev, finalCarry)

incrementer :: [Bit] -> Either String ([Bit], Bit)
incrementer aBits = do
    let bBits = replicate (length aBits) 0
    rippleCarryAdder aBits bBits 1
