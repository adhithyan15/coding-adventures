module BlockRAM.RAM
    ( SinglePortRAM(..)
    , newSinglePortRAM
    , tickSinglePortRAM
    , DualPortRAM(..)
    , newDualPortRAM
    , tickDualPortRAM
    ) where

import LogicGates.Basic
import BlockRAM.SRAM
import BlockRAM.Types

data SinglePortRAM = SinglePortRAM
    { spDepth    :: Int
    , spWidth    :: Int
    , spReadMode :: ReadMode
    , spArray    :: SRAMArray
    , spLastOut  :: [Bit]
    } deriving (Show, Eq)

newSinglePortRAM :: Int -> Int -> ReadMode -> Either String SinglePortRAM
newSinglePortRAM depth width mode = do
    if depth < 1 then Left "Depth must be >= 1"
    else if width < 1 then Left "Width must be >= 1"
    else return $ SinglePortRAM depth width mode (newSRAMArray depth width) (replicate width 0)

tickSinglePortRAM :: SinglePortRAM -> Bit -> Int -> [Bit] -> Bit -> Either String (SinglePortRAM, [Bit])
tickSinglePortRAM ram clock addr dataIn writeEnable = do
    if addr < 0 || addr >= spDepth ram then Left "Address bounds"
    else if clock == 1 then do
        prevData <- readSRAMArray (spArray ram) addr
        let doWrite = writeEnable == 1
        newArr <- if doWrite then writeSRAMArray (spArray ram) addr dataIn else return (spArray ram)
        let outData = case spReadMode ram of
                        ReadFirst -> prevData
                        WriteFirst -> if doWrite then dataIn else prevData
                        NoChange -> if doWrite then spLastOut ram else prevData
        return (ram { spArray = newArr, spLastOut = outData }, outData)
    else 
        return (ram, spLastOut ram)

data DualPortRAM = DualPortRAM
    { dpDepth    :: Int
    , dpWidth    :: Int
    , dpArray    :: SRAMArray
    , dpLastOutA :: [Bit]
    , dpLastOutB :: [Bit]
    } deriving (Show, Eq)

newDualPortRAM :: Int -> Int -> Either String DualPortRAM
newDualPortRAM depth width = do
    if depth < 1 then Left "Depth must be >= 1"
    else if width < 1 then Left "Width must be >= 1"
    else return $ DualPortRAM depth width (newSRAMArray depth width) (replicate width 0) (replicate width 0)

tickDualPortRAM :: DualPortRAM -> Bit -> Int -> [Bit] -> Bit -> Int -> [Bit] -> Bit -> Either String (DualPortRAM, [Bit], [Bit])
tickDualPortRAM ram clock addrA dataInA weA addrB dataInB weB = do
    if clock == 1 then do
        let writeA = weA == 1
        let writeB = weB == 1
        
        if writeA && writeB && addrA == addrB
        then Left "Collision"
        else do
            pA <- readSRAMArray (dpArray ram) addrA
            pB <- readSRAMArray (dpArray ram) addrB
            
            arr1 <- if writeA then writeSRAMArray (dpArray ram) addrA dataInA else return (dpArray ram)
            arr2 <- if writeB then writeSRAMArray arr1 addrB dataInB else return arr1
            
            let outA = if writeA then dataInA else pA
            let outB = if writeB then dataInB else pB
            
            return (ram { dpArray = arr2, dpLastOutA = outA, dpLastOutB = outB }, outA, outB)
    else 
        return (ram, dpLastOutA ram, dpLastOutB ram)
