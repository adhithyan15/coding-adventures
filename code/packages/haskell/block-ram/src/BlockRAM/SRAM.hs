module BlockRAM.SRAM
    ( SRAMCell(..)
    , newSRAMCell
    , readSRAMCell
    , writeSRAMCell
    , SRAMArray(..)
    , newSRAMArray
    , maxRAMBits
    , readSRAMArray
    , writeSRAMArray
    ) where

import LogicGates.Basic
import LogicGates.Sequential (maxDataWidth)

data SRAMCell = SRAMCell
    { cellValue :: Bit
    } deriving (Show, Eq)

newSRAMCell :: SRAMCell
newSRAMCell = SRAMCell 0

readSRAMCell :: SRAMCell -> Bit -> Maybe Bit
readSRAMCell cell wordLine =
    if wordLine == 1 then Just (cellValue cell) else Nothing

writeSRAMCell :: SRAMCell -> Bit -> Bit -> SRAMCell
writeSRAMCell cell wordLine bitLine =
    if wordLine == 1 then cell { cellValue = bitLine } else cell

data SRAMArray = SRAMArray
    { arrayRows  :: Int
    , arrayCols  :: Int
    , arrayCells :: [[SRAMCell]]
    } deriving (Show, Eq)

-- | Largest total RAM (rows * columns, in bits) any constructor accepts.
-- @replicate rows (replicate cols cell)@ allocates eagerly-ish; without a
-- cap a caller-chosen size is a memory-exhaustion DoS.  2^20 bits (128 KiB)
-- is far above anything the educational FPGA model needs.  Widths are also
-- capped at 'maxDataWidth' by 'checkRAMShape'.
maxRAMBits :: Int
maxRAMBits = 2 ^ (20 :: Int)

-- | Shared shape check for every RAM constructor: both dimensions >= 1,
-- width <= 'maxDataWidth', and rows * cols <= 'maxRAMBits'.  The product
-- is computed as a division so it cannot overflow 'Int'.
checkRAMShape :: Int -> Int -> Either LogicGateError ()
checkRAMShape rows cols
    | rows < 1 = Left (InvalidShape "Depth must be >= 1")
    | cols < 1 = Left (InvalidShape "Width must be >= 1")
    | cols > maxDataWidth = Left (InvalidShape "Width exceeds maxDataWidth")
    | rows > maxRAMBits `div` cols = Left (InvalidShape "RAM exceeds maxRAMBits")
    | otherwise = Right ()

-- | Build a zeroed array.  Now total: bad or oversized shapes are an
-- 'InvalidShape' rather than a huge allocation.
newSRAMArray :: Int -> Int -> Either LogicGateError SRAMArray
newSRAMArray rows cols = do
    checkRAMShape rows cols
    return $ SRAMArray rows cols (replicate rows (replicate cols newSRAMCell))

-- | Row lookup without @!!@: out-of-range is a 'Left', never an exception.
rowAt :: SRAMArray -> Int -> Either LogicGateError [SRAMCell]
rowAt arr row
    | row < 0 = Left (InvalidShape "Row index out of bounds")
    | otherwise = case drop row (arrayCells arr) of
        (r:_) -> Right r
        []    -> Left (InvalidShape "Row index out of bounds")

readSRAMArray :: SRAMArray -> Int -> Either LogicGateError [Bit]
readSRAMArray arr row = do
    rowCells <- rowAt arr row
    return (map (\c -> case readSRAMCell c 1 of
                            Just b -> b
                            Nothing -> 0) rowCells)

-- | Write one row.  Defence in depth: data bits are validated here too, so
-- no caller (including a future one that skips the RAM layer's checks) can
-- store a non-bit.
writeSRAMArray :: SRAMArray -> Int -> [Bit] -> Either LogicGateError SRAMArray
writeSRAMArray arr row bits = do
    oldRow <- rowAt arr row
    _ <- validateBits "dataIn" bits
    if length bits /= arrayCols arr
    then Left (InvalidShape "Data length mismatch")
    else do
        let newRow = zipWith (\c b -> writeSRAMCell c 1 b) oldRow bits
        let (before, after) = splitAt row (arrayCells arr)
        return $ arr { arrayCells = before ++ [newRow] ++ drop 1 after }
