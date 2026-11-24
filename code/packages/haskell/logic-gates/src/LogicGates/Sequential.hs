module LogicGates.Sequential
    ( LatchState(..)
    , FlipFlopState(..)
    , srLatch
    , dLatch
    , dFlipFlop
    , register
    , shiftRegister
    , counter
    , maxDataWidth
    , maxCounterWidth
    ) where

import qualified Data.List as L
import LogicGates.Basic

-- | Widest register / shift register accepted (Go lane parity: 65536).
maxDataWidth :: Int
maxDataWidth = 65536

-- | Widest counter accepted (Go lane parity: MaxCounterWidth = 64).
maxCounterWidth :: Int
maxCounterWidth = 64

checkWidth :: String -> Int -> Int -> Either LogicGateError ()
checkWidth name limit n
    | n > limit = Left (InvalidShape (name ++ " width must be at most " ++ show limit))
    | otherwise = Right ()

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

-- | A bank of flip-flops.  @qBits@ and @masterQBits@ must have the same
-- length as @dataBits@ (no silent zero-fill: a mismatch is a caller bug).
-- Width is capped at 'maxDataWidth'.
register :: [Bit] -> Bit -> [Bit] -> [Bit] -> Either LogicGateError ([Bit], [FlipFlopState])
register dataBits clock qBits masterQBits = do
    let n = length dataBits
    checkWidth "dataBits" maxDataWidth n
    if length qBits /= n then Left (InvalidShape "qBits length must equal dataBits length") else Right ()
    if length masterQBits /= n then Left (InvalidShape "masterQBits length must equal dataBits length") else Right ()
    _ <- validateBit "clock" clock
    _ <- validateBits "dataBits" dataBits
    _ <- validateBits "qBits" qBits
    _ <- validateBits "masterQBits" masterQBits
    let eval (d, cq, cmq) = do
            phase1 <- dFlipFlop d 0 cq (1 - cq) cmq (1 - cmq)
            dFlipFlop d clock (ffQ phase1) (ffQBar phase1) (ffMasterQ phase1) (1 - ffMasterQ phase1)

    states <- mapM eval (zip3 dataBits qBits masterQBits)
    return (map ffQ states, states)

shiftRegister :: Bit -> Bit -> [Bit] -> Either LogicGateError ([Bit], Bit, [FlipFlopState])
shiftRegister serialIn clock qBits = do
    checkWidth "qBits" maxDataWidth (length qBits)
    _ <- validateBit "serialIn" serialIn
    _ <- validateBit "clock" clock
    _ <- validateBits "qBits" qBits
    let currentQ = if null qBits then replicate 4 0 else qBits
    let dataBits = serialIn : take (length currentQ - 1) currentQ

    let eval (d, cq) = do
            phase1 <- dFlipFlop d 0 cq (1 - cq) 0 1
            dFlipFlop d clock (ffQ phase1) (ffQBar phase1) (ffMasterQ phase1) (1 - ffMasterQ phase1)

    states <- mapM eval (zip dataBits currentQ)
    let newQ = map ffQ states
    let serialOut = case reverse newQ of
            (x : _) -> x
            [] -> 0
    return (newQ, serialOut, states)

counter :: Bit -> Bit -> [Bit] -> Either LogicGateError ([Bit], Bit, [FlipFlopState])
counter clock reset qBits = do
    checkWidth "qBits" maxCounterWidth (length qBits)
    _ <- validateBit "clock" clock
    _ <- validateBit "reset" reset
    _ <- validateBits "qBits" qBits
    let currentQ = if null qBits then replicate 4 0 else qBits
    let len = length currentQ

    let (nextCount, overflow) = if reset == 1
                                then (replicate len 0, 0)
                                else foldlInc currentQ len

    (newQ, states) <- register nextCount clock currentQ (replicate len 0)
    return (newQ, overflow, states)

foldlInc :: [Bit] -> Int -> ([Bit], Bit)
foldlInc q _ = 
    let step carry bit = let sumBit = bit + carry in (sumBit `mod` 2, sumBit `div` 2)
        revQ = reverse q
        (revNext, finalCarry) = L.foldl' (\(acc, c) b -> let (nb, nc) = step c b in (nb:acc, nc)) ([], 1) revQ
    in (reverse revNext, finalCarry)
