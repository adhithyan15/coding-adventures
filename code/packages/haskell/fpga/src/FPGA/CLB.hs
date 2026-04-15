module FPGA.CLB
    ( CLBOutput(..)
    , CLB(..)
    , newCLB
    , evaluateCLB
    ) where

import LogicGates.Basic
import FPGA.Slice

data CLBOutput = CLBOutput
    { clbOut0 :: SliceOutput
    , clbOut1 :: SliceOutput
    } deriving (Show, Eq)

data CLB = CLB
    { clbK      :: Int
    , clbSlice0 :: Slice
    , clbSlice1 :: Slice
    } deriving (Show, Eq)

newCLB :: Int -> Either String CLB
newCLB k = do
    s0 <- newSlice k
    s1 <- newSlice k
    return $ CLB k s0 s1

evaluateCLB :: CLB -> [Bit] -> [Bit] -> [Bit] -> [Bit] -> Bit -> Bit -> Either String (CLB, CLBOutput)
evaluateCLB clb s0inA s0inB s1inA s1inB clock cIn = do
    (ns0, out0) <- evaluateSlice (clbSlice0 clb) s0inA s0inB clock cIn
    (ns1, out1) <- evaluateSlice (clbSlice1 clb) s1inA s1inB clock (carryOut out0)
    return (clb { clbSlice0 = ns0, clbSlice1 = ns1 }, CLBOutput out0 out1)
