{-# LANGUAGE Safe #-}
module LogicGates.Combinational
    ( mux2
    , mux4
    , mux8
    , muxN
    , maxSelectBits
    , demux
    , decoder
    , encoder
    , priorityEncoder
    , triState
    , maxInputWidth
    ) where

import LogicGates.Basic

mux2 :: Bit -> Bit -> Bit -> Either LogicGateError Bit
mux2 d0 d1 sel = do
    _ <- validateBit "d0" d0
    _ <- validateBit "d1" d1
    _ <- validateBit "sel" sel
    return $ if sel == 1 then d1 else d0

-- | 4-to-1 mux.  Note the ordering: the shape of @sel@ is checked BEFORE any
-- bit is validated, so a wrong-length @sel@ holding a non-bit reports
-- 'InvalidShape', not 'InvalidBit'.
mux4 :: Bit -> Bit -> Bit -> Bit -> [Bit] -> Either LogicGateError Bit
mux4 d0 d1 d2 d3 selIn = do
    sel <- exactLength "sel" 2 selIn
    _ <- validateBit "d0" d0
    _ <- validateBit "d1" d1
    _ <- validateBit "d2" d2
    _ <- validateBit "d3" d3
    _ <- validateBits "sel" sel
    case sel of
        [s0, s1] -> do
            low <- mux2 d0 d1 s0
            high <- mux2 d2 d3 s0
            mux2 low high s1
        _ -> Left (InvalidShape "sel must have length 2")

mux8 :: [Bit] -> [Bit] -> Either LogicGateError Bit
mux8 inputsIn selIn = do
    inputs <- exactLength "inputs" 8 inputsIn
    sel <- exactLength "sel" 3 selIn
    muxN inputs sel

-- | Largest number of select bits accepted by 'muxN', 'demux' and
-- 'decoder'.  These build @2 ^ n@ lines, so an unbounded @n@ is a
-- denial-of-service (and @2 ^ 64@ wraps to 0 in an 'Int').  Parity with the
-- Rust (SEC-008) and Go (MaxSelectBits) lanes.
maxSelectBits :: Int
maxSelectBits = 16

-- | Largest number of inputs accepted by 'muxN', 'encoder' and
-- 'priorityEncoder' (@2 ^ maxSelectBits@).
maxInputWidth :: Int
maxInputWidth = 65536

-- | Bounded snapshot of a list that must have exactly @n@ elements.  Walks at
-- most @n + 1@ cells, so an infinite list is rejected, not traversed.
exactLength :: String -> Int -> [Bit] -> Either LogicGateError [Bit]
exactLength name n xs = do
    ys <- takeChecked name (n + 1) xs
    if length ys /= n
    then Left (InvalidShape (name ++ " must have length " ++ show n))
    else Right ys

-- | Snapshot a select list: at most 'maxSelectBits' elements.
snapSel :: String -> [Bit] -> Either LogicGateError [Bit]
snapSel name xs =
    case takeChecked name (maxSelectBits + 1) xs of
        Right ys | length ys > maxSelectBits -> Left (selTooWide name)
        Right ys -> Right ys
        Left _ -> Left (selTooWide name)

selTooWide :: String -> LogicGateError
selTooWide name = InvalidShape (name ++ " must have at most " ++ show maxSelectBits ++ " bits")

-- | N-to-1 multiplexer.  @inputs@ must have exactly @2 ^ length sel@
-- entries and @sel@ between 1 and 'maxSelectBits' bits (little-endian: sel[0]
-- is the least significant).  Total: every bad shape is an 'InvalidShape'.
--
-- Each select bit, lowest first, halves the list by choosing one of every
-- adjacent pair; after all bits exactly one value is left.
muxN :: [Bit] -> [Bit] -> Either LogicGateError Bit
muxN inputsIn selIn = do
    sel <- snapSel "sel" selIn
    inputs <- takeChecked "inputs" maxInputWidth inputsIn
    _ <- validateBits "inputs" inputs
    _ <- validateBits "sel" sel
    if null sel
    then Left (InvalidShape "sel must have at least 1 bit")
    else if length inputs /= 2 ^ length sel
    then Left (InvalidShape "inputs length must equal 2 ^ length sel")
    else case foldl (flip muxStep) inputs sel of
        [x] -> Right x
        _ -> Left (InvalidShape "inputs length must equal 2 ^ length sel")
  where
    muxStep s (x : y : rest) = (if s == 1 then y else x) : muxStep s rest
    muxStep _ _ = []

demux :: Bit -> [Bit] -> Either LogicGateError [Bit]
demux dataBit selIn = do
    sel <- snapSel "sel" selIn
    _ <- validateBit "dataBit" dataBit
    _ <- validateBits "sel" sel
    let nOutputs = (2 :: Int) ^ length sel
    let idx = foldr (\(i, s) acc -> acc + s * (2 ^ i)) 0 (zip [0 :: Int ..] sel)
    return $ forceBits [if (i :: Int) == idx then dataBit else 0 | i <- [0..(nOutputs - 1)]]

decoder :: [Bit] -> Either LogicGateError [Bit]
decoder inputsIn = do
    inputs <- snapSel "inputs" inputsIn
    _ <- validateBits "inputs" inputs
    let nOutputs = (2 :: Int) ^ length inputs
    let idx = foldr (\(i, b) acc -> acc + b * (2 ^ i)) 0 (zip [0 :: Int ..] inputs)
    return $ forceBits [if (i :: Int) == idx then 1 else 0 | i <- [0..(nOutputs - 1)]]

encoder :: [Bit] -> Either LogicGateError [Bit]
encoder inputsIn = do
    inputs <- takeChecked "inputs" maxInputWidth inputsIn
    _ <- validateBits "inputs" inputs
    let active = filter (\(_, v) -> v == 1) (zip [0 :: Int ..] inputs)
    case active of
        [(idx, _)] -> do
            let n = log2Ceil (length inputs)
            return $ forceBits [(idx `div` (2 ^ bit)) `mod` 2 | bit <- [0..(n - 1)]]
        _ -> Left (InvalidShape "exactly one input must be 1")

priorityEncoder :: [Bit] -> Either LogicGateError ([Bit], Bit)
priorityEncoder inputsIn = do
    inputs <- takeChecked "inputs" maxInputWidth inputsIn
    _ <- validateBits "inputs" inputs
    let active = filter (\(_, v) -> v == 1) (reverse $ zip [0..] inputs)
    let n = log2Ceil (length inputs)
    case active of
        [] -> return (replicate n 0, 0)
        ((idx, _) : _) -> do
            let out = forceBits [(idx `div` (2 ^ bit)) `mod` 2 | bit <- [0..(n - 1)]]
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
