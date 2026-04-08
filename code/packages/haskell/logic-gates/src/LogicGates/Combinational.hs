module LogicGates.Combinational
    ( mux2
    , mux4
    , mux8
    , muxN
    , demux
    , decoder
    , encoder
    , priorityEncoder
    , triState
    ) where

import LogicGates.Basic

mux2 :: Bit -> Bit -> Bit -> Either String Bit
mux2 d0 d1 sel = do
    _ <- validateBit d0
    _ <- validateBit d1
    _ <- validateBit sel
    return $ if sel == 1 then d1 else d0

mux4 :: Bit -> Bit -> Bit -> Bit -> [Bit] -> Either String Bit
mux4 d0 d1 d2 d3 sel = do
    if length sel /= 2 then Left "sel must have length 2" else Right ()
    mapM_ validateBit (d0 : d1 : d2 : d3 : sel)
    s0 <- return $ sel !! 0
    s1 <- return $ sel !! 1
    low <- mux2 d0 d1 s0
    high <- mux2 d2 d3 s0
    mux2 low high s1

mux8 :: [Bit] -> [Bit] -> Either String Bit
mux8 inputs sel = do
    if length inputs /= 8 then Left "inputs must have length 8" else Right ()
    if length sel /= 3 then Left "sel must have length 3" else Right ()
    muxN inputs sel

muxN :: [Bit] -> [Bit] -> Either String Bit
muxN inputs sel = do
    mapM_ validateBit inputs
    mapM_ validateBit sel
    if length inputs <= 2
    then
        if length inputs == 2 && length sel >= 1
        then mux2 (inputs !! 0) (inputs !! 1) (sel !! 0)
        else Left "Insufficient inputs or sel length"
    else do
        let half = length inputs `div` 2
        let lowerSel = init sel
        lo <- muxN (take half inputs) lowerSel
        hi <- muxN (drop half inputs) lowerSel
        mux2 lo hi (last sel)

demux :: Bit -> [Bit] -> Either String [Bit]
demux dataBit sel = do
    _ <- validateBit dataBit
    mapM_ validateBit sel
    let nOutputs = (1 :: Int) * (2 ^ length sel)
    let idx = foldr (\(i, s) acc -> acc + s * (2 ^ i)) 0 (zip [0..] sel)
    return $ [if (i :: Int) == idx then dataBit else 0 | i <- [0..(nOutputs - 1)]]

decoder :: [Bit] -> Either String [Bit]
decoder inputs = do
    mapM_ validateBit inputs
    let nOutputs = (1 :: Int) * (2 ^ length inputs)
    let idx = foldr (\(i, b) acc -> acc + b * (2 ^ i)) 0 (zip [0..] inputs)
    return $ [if (i :: Int) == idx then 1 else 0 | i <- [0..(nOutputs - 1)]]

encoder :: [Bit] -> Either String [Bit]
encoder inputs = do
    mapM_ validateBit inputs
    let active = filter (\(_, v) -> v == 1) (zip [0..] inputs)
    if length active /= 1
    then Left "exactly one input must be 1"
    else do
        let (idx, _) = head active
        let n = log2Ceil (length inputs)
        return $ [(idx `div` (2 ^ bit)) `mod` 2 | bit <- [0..(n - 1)]]

priorityEncoder :: [Bit] -> Either String ([Bit], Bit)
priorityEncoder inputs = do
    mapM_ validateBit inputs
    let active = filter (\(_, v) -> v == 1) (reverse $ zip [0..] inputs)
    let n = log2Ceil (length inputs)
    if null active
    then return (replicate n 0, 0)
    else do
        let (idx, _) = head active
        let out = [(idx `div` (2 ^ bit)) `mod` 2 | bit <- [0..(n - 1)]]
        return (out, 1)

triState :: Bit -> Bit -> Either String (Maybe Bit)
triState dataBit enable = do
    _ <- validateBit dataBit
    _ <- validateBit enable
    return $ if enable == 1 then Just dataBit else Nothing

log2Ceil :: Int -> Int
log2Ceil n | n <= 1 = 0
           | otherwise = go (n - 1) 0
  where
    go 0 bits = bits
    go v bits = go (v `div` 2) (bits + 1)
