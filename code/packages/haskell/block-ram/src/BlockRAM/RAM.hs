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

newSinglePortRAM :: Int -> Int -> ReadMode -> Either LogicGateError SinglePortRAM
newSinglePortRAM depth width mode = do
    arr <- newSRAMArray depth width
    return $ SinglePortRAM depth width mode arr (replicate width 0)

tickSinglePortRAM :: SinglePortRAM -> Bit -> Int -> [Bit] -> Bit -> Either LogicGateError (SinglePortRAM, [Bit])
tickSinglePortRAM ram clock addr dataIn writeEnable = do
    -- Validate every input, whether or not this tick writes: a bad
    -- writeEnable (2) used to mean "silently no write", and a bad dataIn
    -- was stored as-is.
    _ <- validateBit "clock" clock
    _ <- validateBit "writeEnable" writeEnable
    _ <- validateBits "dataIn" dataIn
    if length dataIn /= spWidth ram then Left (InvalidShape "Data length mismatch")
    else if addr < 0 || addr >= spDepth ram then Left (InvalidShape "Address bounds")
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

newDualPortRAM :: Int -> Int -> Either LogicGateError DualPortRAM
newDualPortRAM depth width = do
    arr <- newSRAMArray depth width
    return $ DualPortRAM depth width arr (replicate width 0) (replicate width 0)

tickDualPortRAM :: DualPortRAM -> Bit -> Int -> [Bit] -> Bit -> Int -> [Bit] -> Bit -> Either LogicGateError (DualPortRAM, [Bit], [Bit])
tickDualPortRAM ram clock addrA dataInA weA addrB dataInB weB = do
    _ <- validateBit "clock" clock
    _ <- validateBit "weA" weA
    _ <- validateBit "weB" weB
    _ <- validateBits "dataInA" dataInA
    _ <- validateBits "dataInB" dataInB
    if length dataInA /= dpWidth ram || length dataInB /= dpWidth ram
    then Left (InvalidShape "Data length mismatch")
    else if addrA < 0 || addrA >= dpDepth ram || addrB < 0 || addrB >= dpDepth ram
    then Left (InvalidShape "Address bounds")
    else if clock == 1 then do
        let writeA = weA == 1
        let writeB = weB == 1
        
        if writeA && writeB && addrA == addrB
        then Left (InvalidShape "Collision")
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
