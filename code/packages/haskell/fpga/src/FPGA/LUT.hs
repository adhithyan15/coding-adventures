module FPGA.LUT
    ( LUT(..)
    , newLUT
    , configureLUT
    , evaluateLUT
    ) where

import LogicGates
import BlockRAM

data LUT = LUT
    { lutK    :: Int
    , lutSize :: Int
    , lutSRAM :: [SRAMCell]
    } deriving (Show, Eq)

newLUT :: Int -> Either LogicGateError LUT
newLUT k = do
    if k < 2 || k > 6 then Left (InvalidShape "k must be between 2 and 6")
    else Right $ LUT k (2^k) (replicate (2^k) newSRAMCell)

configureLUT :: LUT -> [Bit] -> Either LogicGateError LUT
configureLUT lut truthTable = do
    -- Validate the table itself first (param "truthTable"), so a bad bit
    -- is not later misreported as a bad evaluation input.  The expected
    -- length is 2^k; k is bounded by maxSelectBits so the size is sane
    -- even for a hand-built LUT record.
    _ <- validateBits "truthTable" truthTable
    if lutK lut < 1 || lutK lut > maxSelectBits || lutSize lut /= 2 ^ lutK lut
    then Left (InvalidShape "LUT size must be 2^k with k <= maxSelectBits")
    else if length truthTable /= lutSize lut
    then Left (InvalidShape "truthTable length must match 2^k")
    else do
        let newSram = zipWith (\cell bit -> writeSRAMCell cell 1 bit) (lutSRAM lut) truthTable
        Right $ lut { lutSRAM = newSram }

evaluateLUT :: LUT -> [Bit] -> Either LogicGateError Bit
evaluateLUT lut inputs = do
    if length inputs /= lutK lut
    then Left (InvalidShape "inputs length must match k")
    else do
        let dataBits = map (\c -> case readSRAMCell c 1 of Just b -> b; Nothing -> 0) (lutSRAM lut)
        muxN dataBits inputs
