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

mux2 :: Bit -> Bit -> Bit -> Either LogicGateError Bit
mux2 d0 d1 sel = do
    _ <- validateBit "d0" d0
    _ <- validateBit "d1" d1
    _ <- validateBit "sel" sel
    return $ if sel == 1 then d1 else d0

mux4 :: Bit -> Bit -> Bit -> Bit -> [Bit] -> Either LogicGateError Bit
mux4 d0 d1 d2 d3 sel = do
    if length sel /= 2 then Left (InvalidShape "sel must have length 2") else Right ()
    _ <- validateBit "d0" d0
    _ <- validateBit "d1" d1
    _ <- validateBit "d2" d2
    _ <- validateBit "d3" d3
    _ <- validateBits "sel" sel
    s0 <- return $ sel !! 0
    s1 <- return $ sel !! 1
    low <- mux2 d0 d1 s0
    high <- mux2 d2 d3 s0
    mux2 low high s1

mux8 :: [Bit] -> [Bit] -> Either LogicGateError Bit
mux8 inputs sel = do
    if length inputs /= 8 then Left (InvalidShape "inputs must have length 8") else Right ()
    if length sel /= 3 then Left (InvalidShape "sel must have length 3") else Right ()
    muxN inputs sel

muxN :: [Bit] -> [Bit] -> Either LogicGateError Bit
muxN inputs sel = do
    _ <- validateBits "inputs" inputs
    _ <- validateBits "sel" sel
    if length inputs <= 2
    then
        if length inputs == 2 && length sel >= 1
        then mux2 (inputs !! 0) (inputs !! 1) (sel !! 0)
        else Left (InvalidShape "Insufficient inputs or sel length")
    else do
        let half = length inputs `div` 2
        let lowerSel = init sel
        lo <- muxN (take half inputs) lowerSel
        hi <- muxN (drop half inputs) lowerSel
        mux2 lo hi (last sel)

demux :: Bit -> [Bit] -> Either LogicGateError [Bit]
demux dataBit sel = do
    _ <- validateBit "dataBit" dataBit
    _ <- validateBits "sel" sel
    let nOutputs = (1 :: Int) * (2 ^ length sel)
    let idx = foldr (\(i, s) acc -> acc + s * (2 ^ i)) 0 (zip [0 :: Int ..] sel)
    return $ [if (i :: Int) == idx then dataBit else 0 | i <- [0..(nOutputs - 1)]]

decoder :: [Bit] -> Either LogicGateError [Bit]
decoder inputs = do
    _ <- validateBits "inputs" inputs
    let nOutputs = (1 :: Int) * (2 ^ length inputs)
    let idx = foldr (\(i, b) acc -> acc + b * (2 ^ i)) 0 (zip [0 :: Int ..] inputs)
    return $ [if (i :: Int) == idx then 1 else 0 | i <- [0..(nOutputs - 1)]]

encoder :: [Bit] -> Either LogicGateError [Bit]
encoder inputs = do
    _ <- validateBits "inputs" inputs
    let active = filter (\(_, v) -> v == 1) (zip [0 :: Int ..] inputs)
    case active of
        [(idx, _)] -> do
            let n = log2Ceil (length inputs)
            return $ [(idx `div` (2 ^ bit)) `mod` 2 | bit <- [0..(n - 1)]]
        _ -> Left (InvalidShape "exactly one input must be 1")

priorityEncoder :: [Bit] -> Either LogicGateError ([Bit], Bit)
priorityEncoder inputs = do
    _ <- validateBits "inputs" inputs
    let active = filter (\(_, v) -> v == 1) (reverse $ zip [0..] inputs)
    let n = log2Ceil (length inputs)
    case active of
        [] -> return (replicate n 0, 0)
        ((idx, _) : _) -> do
            let out = [(idx `div` (2 ^ bit)) `mod` 2 | bit <- [0..(n - 1)]]
            return (out, 1)

triState :: Bit -> Bit -> Either LogicGateError (Maybe Bit)
triState dataBit enable = do
    _ <- validateBit "dataBit" dataBit
    _ <- validateBit "enable" enable
    return $ if enable == 1 then Just dataBit else Nothing

log2Ceil :: Int -> Int
log2Ceil n | n <= 1 = 0
           | otherwise = go (n - 1) 0
  where
    go 0 bits = bits
    go v bits = go (v `div` 2) (bits + 1)
