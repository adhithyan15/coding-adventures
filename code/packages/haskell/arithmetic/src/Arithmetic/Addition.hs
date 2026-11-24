module Arithmetic.Addition
    ( halfAdder
    , fullAdder
    , rippleCarryAdder
    , incrementer
    ) where

import LogicGates

halfAdder :: Bit -> Bit -> Either LogicGateError (Bit, Bit)
halfAdder a b = do
    sumBit <- xorGate a b
    carryOut <- andGate a b
    return (sumBit, carryOut)

fullAdder :: Bit -> Bit -> Bit -> Either LogicGateError (Bit, Bit)
fullAdder a b carryIn = do
    (sum1, carry1) <- halfAdder a b
    (sumOut, carry2) <- halfAdder sum1 carryIn
    carryOut <- orGate carry1 carry2
    return (sumOut, carryOut)

rippleCarryAdder :: [Bit] -> [Bit] -> Bit -> Either LogicGateError ([Bit], Bit)
rippleCarryAdder aBits bBits carryIn = do
    -- Validate everything up front: with empty operands the carry would
    -- otherwise flow straight to the output unchecked.
    _ <- validateBit "carryIn" carryIn
    _ <- validateBits "a" aBits
    _ <- validateBits "b" bBits
    if length aBits /= length bBits
    then Left (InvalidShape "Inputs must have the same length")
    else do
        let go cOut [] [] = return ([], cOut)
            go cOut (a:as) (b:bs) = do
                (sumOut, nextC) <- fullAdder a b cOut
                (restSum, finalC) <- go nextC as bs
                return (sumOut : restSum, finalC)
            go _ _ _ = Left (InvalidShape "Length mismatch")
        (sumRev, finalCarry) <- go carryIn (reverse aBits) (reverse bBits)
        return (reverse sumRev, finalCarry)

incrementer :: [Bit] -> Either LogicGateError ([Bit], Bit)
incrementer aBits = do
    let bBits = replicate (length aBits) 0
    rippleCarryAdder aBits bBits 1
