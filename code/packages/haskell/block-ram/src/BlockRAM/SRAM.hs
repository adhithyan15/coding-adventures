module BlockRAM.SRAM
    ( SRAMCell(..)
    , newSRAMCell
    , readSRAMCell
    , writeSRAMCell
    , SRAMArray(..)
    , newSRAMArray
    , readSRAMArray
    , writeSRAMArray
    ) where

import LogicGates.Basic

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

newSRAMArray :: Int -> Int -> SRAMArray
newSRAMArray rows cols = 
    SRAMArray rows cols (replicate rows (replicate cols newSRAMCell))

readSRAMArray :: SRAMArray -> Int -> Either String [Bit]
readSRAMArray arr row = do
    if row < 0 || row >= arrayRows arr
    then Left "Row index out of bounds"
    else do
        let rowCells = arrayCells arr !! row
        let bits = map (\c -> case readSRAMCell c 1 of
                                Just b -> b
                                Nothing -> 0) rowCells
        return bits

writeSRAMArray :: SRAMArray -> Int -> [Bit] -> Either String SRAMArray
writeSRAMArray arr row bits = do
    if row < 0 || row >= arrayRows arr
    then Left "Row index out of bounds"
    else if length bits /= arrayCols arr
    then Left "Data length mismatch"
    else do
        let updateCell c b = writeSRAMCell c 1 b
        let oldRow = arrayCells arr !! row
        let newRow = zipWith updateCell oldRow bits
        let newCells = take row (arrayCells arr) ++ [newRow] ++ drop (row + 1) (arrayCells arr)
        return $ arr { arrayCells = newCells }
