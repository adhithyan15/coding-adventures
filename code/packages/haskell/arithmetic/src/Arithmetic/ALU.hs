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

-- Evaluates the ALU operation based exactly on combinational rules.
alu :: [Bit] -> [Bit] -> ALUControl -> Either String ALUOutput
alu aBits bBits control = do
    if length aBits /= length bBits
    then Left "Data inputs must have same length"
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

        let finalRes = case control of
                AND -> andRes
                OR  -> orRes
                XOR -> xorRes
                SLT -> let nBit = head subRes
                           vBit = overflowCalc (head aBits) (head (map (1-) bBits)) nBit
                       in replicate (length aBits - 1) 0 ++ [xorGatePure nBit vBit]
                _   -> arithRes

        zero <- isZero finalRes
        let nBit = head finalRes
        
        let addV = overflowCalc (head aBits) (head bBits) (head addRes)
        let subV = overflowCalc (head aBits) (1 - head bBits) (head subRes)
        let vOut = if isSub then subV else addV

        return $ ALUOutput finalRes (ALUFlags zero nBit cOut vOut)
        
  where
    xorGatePure a b = if a /= b then 1 else 0
    overflowCalc aMSB bMSB rMSB = if aMSB == bMSB && rMSB /= aMSB then 1 else 0
