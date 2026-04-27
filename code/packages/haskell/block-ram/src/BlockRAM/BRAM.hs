module BlockRAM.BRAM
    ( ConfigurableBRAM(..)
    , newBRAM
    , reconfigureBRAM
    , tickBRAM_A
    , tickBRAM_B
    ) where

import LogicGates.Basic
import BlockRAM.RAM

data ConfigurableBRAM = ConfigurableBRAM
    { cbTotalBits :: Int
    , cbWidth     :: Int
    , cbDepth     :: Int
    , cbRam       :: DualPortRAM
    } deriving (Show, Eq)

newBRAM :: Int -> Int -> Either String ConfigurableBRAM
newBRAM totalBits width = do
    let depth = totalBits `div` width
    ram <- newDualPortRAM depth width
    return $ ConfigurableBRAM totalBits width depth ram

reconfigureBRAM :: ConfigurableBRAM -> Int -> Either String ConfigurableBRAM
reconfigureBRAM bram newWidth = do
    newBRAM (cbTotalBits bram) newWidth

tickBRAM_A :: ConfigurableBRAM -> Bit -> Int -> [Bit] -> Bit -> Either String (ConfigurableBRAM, [Bit])
tickBRAM_A bram clock addr dataIn we = do
    let zeros = replicate (cbWidth bram) 0
    (nram, outA, _) <- tickDualPortRAM (cbRam bram) clock addr dataIn we 0 zeros 0
    return (bram { cbRam = nram }, outA)

tickBRAM_B :: ConfigurableBRAM -> Bit -> Int -> [Bit] -> Bit -> Either String (ConfigurableBRAM, [Bit])
tickBRAM_B bram clock addr dataIn we = do
    let zeros = replicate (cbWidth bram) 0
    (nram, _, outB) <- tickDualPortRAM (cbRam bram) clock 0 zeros 0 addr dataIn we
    return (bram { cbRam = nram }, outB)
