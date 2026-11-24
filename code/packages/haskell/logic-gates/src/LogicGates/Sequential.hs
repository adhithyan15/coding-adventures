module LogicGates.Sequential
    ( LatchState(..)
    , FlipFlopState(..)
    , srLatch
    , dLatch
    , dFlipFlop
    , register
    , shiftRegister
    , counter
    ) where

import LogicGates.Basic

data LatchState = LatchState
    { qOut    :: Bit
    , qBarOut :: Bit
    } deriving (Show, Eq)

data FlipFlopState = FlipFlopState
    { ffQ       :: Bit
    , ffQBar    :: Bit
    , ffMasterQ :: Bit
    } deriving (Show, Eq)

srLatch :: Bit -> Bit -> Bit -> Bit -> Either LogicGateError LatchState
srLatch s r q qBar = do
    _ <- validateBit "s" s
    _ <- validateBit "r" r
    _ <- validateBit "q" q
    _ <- validateBit "qBar" qBar
    let loop curQ curQBar count = do
            if count >= (3 :: Int)
            then return (curQ, curQBar)
            else do
                newQ <- norGate r curQBar
                newQBar <- norGate s newQ
                if newQ == curQ && newQBar == curQBar
                then return (newQ, newQBar)
                else loop newQ newQBar (count + 1)
    (finalQ, finalQBar) <- loop q qBar 0
    return $ LatchState finalQ finalQBar

dLatch :: Bit -> Bit -> Bit -> Bit -> Either LogicGateError LatchState
dLatch d e q qBar = do
    _ <- validateBit "d" d
    _ <- validateBit "e" e
    _ <- validateBit "q" q
    _ <- validateBit "qBar" qBar
    s <- andGate d e
    notD <- notGate d
    r <- andGate notD e
    srLatch s r q qBar

dFlipFlop :: Bit -> Bit -> Bit -> Bit -> Bit -> Bit -> Either LogicGateError FlipFlopState
dFlipFlop d clock q qBar masterQ masterQBar = do
    _ <- validateBit "d" d
    _ <- validateBit "clock" clock
    _ <- validateBit "q" q
    _ <- validateBit "qBar" qBar
    _ <- validateBit "masterQ" masterQ
    _ <- validateBit "masterQBar" masterQBar
    clockBar <- notGate clock
    master <- dLatch d clockBar masterQ masterQBar
    slave <- dLatch (qOut master) clock q qBar
    return $ FlipFlopState (qOut slave) (qBarOut slave) (qOut master)

register :: [Bit] -> Bit -> [Bit] -> [Bit] -> Either LogicGateError ([Bit], [FlipFlopState])
register dataBits clock qBits masterQBits = do
    let n = length dataBits
    let currentQ = if length qBits == n then qBits else replicate n 0
    let currentMQ = if length masterQBits == n then masterQBits else replicate n 0
    _ <- validateBit "clock" clock
    _ <- validateBits "dataBits" dataBits
    _ <- validateBits "qBits" currentQ
    _ <- validateBits "masterQBits" currentMQ
    let eval i = do
            let d = dataBits !! i
            let cq = currentQ !! i
            let cmq = currentMQ !! i
            phase1 <- dFlipFlop d 0 cq (1 - cq) cmq (1 - cmq)
            s <- dFlipFlop d clock (ffQ phase1) (ffQBar phase1) (ffMasterQ phase1) (1 - ffMasterQ phase1)
            return s

    states <- mapM eval [0..(n - 1)]
    return (map ffQ states, states)

shiftRegister :: Bit -> Bit -> [Bit] -> Either LogicGateError ([Bit], Bit, [FlipFlopState])
shiftRegister serialIn clock qBits = do
    _ <- validateBit "serialIn" serialIn
    _ <- validateBit "clock" clock
    _ <- validateBits "qBits" qBits
    let n = length qBits
    let currentQ = if n > 0 then qBits else replicate 4 0
    let len = length currentQ
    let dataBits = serialIn : init currentQ

    let eval i = do
            let d = dataBits !! i
            let cq = currentQ !! i
            phase1 <- dFlipFlop d 0 cq (1 - cq) 0 1
            s <- dFlipFlop d clock (ffQ phase1) (ffQBar phase1) (ffMasterQ phase1) (1 - ffMasterQ phase1)
            return s

    states <- mapM eval [0..(len - 1)]
    let newQ = map ffQ states
    let serialOut = last newQ
    return (newQ, serialOut, states)

counter :: Bit -> Bit -> [Bit] -> Either LogicGateError ([Bit], Bit, [FlipFlopState])
counter clock reset qBits = do
    _ <- validateBit "clock" clock
    _ <- validateBit "reset" reset
    _ <- validateBits "qBits" qBits
    let n = length qBits
    let currentQ = if n > 0 then qBits else replicate 4 0
    let len = length currentQ

    let (nextCount, overflow) = if reset == 1
                                then (replicate len 0, 0)
                                else foldlInc currentQ len
    
    (newQ, states) <- register nextCount clock currentQ []
    return (newQ, overflow, states)

foldlInc :: [Bit] -> Int -> ([Bit], Bit)
foldlInc q _ = 
    let step carry bit = let sumBit = bit + carry in (sumBit `mod` 2, sumBit `div` 2)
        revQ = reverse q
        (revNext, finalCarry) = foldl (\(acc, c) b -> let (nb, nc) = step c b in (nb:acc, nc)) ([], 1) revQ
    in (reverse revNext, finalCarry)
