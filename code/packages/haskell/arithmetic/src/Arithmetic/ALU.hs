module Arithmetic.ALU
    ( ALUFlags(..)
    , ALUOutput(..)
    , ALUControl(..)
    , alu
    ) where

import LogicGates
import Arithmetic.Addition
import Arithmetic.Subtraction
import Arithmetic.Comparison

data ALUFlags = ALUFlags
    { zFlag :: Bit
    , nFlag :: Bit
    , cFlag :: Bit
    , vFlag :: Bit
    } deriving (Show, Eq)

data ALUOutput = ALUOutput
    { result :: [Bit]
    , flags  :: ALUFlags
    } deriving (Show, Eq)

data ALUControl = ADD | SUB | AND | OR | XOR | SLT deriving (Show, Eq)

-- | Most significant bit (the sign bit in two's complement) of a bit
-- vector, as a total function.  The ALU used to call @head@ here, which
-- throws on an empty list *lazily* -- the error escaped the 'Either' the
-- first time a caller read a flag.  'alu' now rejects empty operands up
-- front, and this helper turns "impossible" emptiness into a proper
-- 'InvalidShape' instead of a crash.
msb :: [Bit] -> Either LogicGateError Bit
msb (b:_) = Right b
msb []    = Left (InvalidShape "Data inputs must not be empty")

-- | Evaluates the ALU operation based exactly on combinational rules.
--
-- Validation order: operand bits (@a@, then @b@), then shape (non-empty,
-- equal length).  No partial list function is used anywhere on this path.
alu :: [Bit] -> [Bit] -> ALUControl -> Either LogicGateError ALUOutput
alu aBits bBits control = do
    _ <- validateBits "a" aBits
    _ <- validateBits "b" bBits
    if null aBits || null bBits
    then Left (InvalidShape "Data inputs must not be empty")
    else if length aBits /= length bBits
    then Left (InvalidShape "Data inputs must have same length")
    else do
        let isSub = control == SUB || control == SLT

        -- Logic operations
        andRes <- mapM (uncurry andGate) (zip aBits bBits)
        orRes  <- mapM (uncurry orGate) (zip aBits bBits)
        xorRes <- mapM (uncurry xorGate) (zip aBits bBits)

        -- Arithmetic operations
        (addRes, addC) <- rippleCarryAdder aBits bBits 0
        (subRes, subB) <- subtractorN aBits bBits
        let subC = 1 - subB

        let arithRes = if isSub then subRes else addRes
        let cOut     = if isSub then subC else addC

        aMSB   <- msb aBits
        bMSB   <- msb bBits
        addMSB <- msb addRes
        subMSB <- msb subRes

        let subV = overflowCalc aMSB (1 - bMSB) subMSB
        let addV = overflowCalc aMSB bMSB addMSB

        let finalRes = case control of
                AND -> andRes
                OR  -> orRes
                XOR -> xorRes
                SLT -> replicate (length aBits - 1) 0 ++ [xorGatePure subMSB subV]
                _   -> arithRes

        zero <- isZero finalRes
        nBit <- msb finalRes
        let vOut = if isSub then subV else addV

        return $ ALUOutput finalRes (ALUFlags zero nBit cOut vOut)

  where
    xorGatePure a b = if a /= b then 1 else 0
    overflowCalc aM bM rM = if aM == bM && rM /= aM then 1 else 0
