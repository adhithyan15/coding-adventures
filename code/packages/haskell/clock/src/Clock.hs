module Clock
    ( ClockState(..)
    , newClock
    , tick
    , ClockDivider(..)
    , newClockDivider
    , tickDivider
    , MultiPhaseClock(..)
    , newMultiPhaseClock
    , tickMultiPhase
    ) where

import LogicGates (Bit)

data ClockState = ClockState
    { clockValue :: Bit
    , cycleCount :: Int
    } deriving (Show, Eq)

newClock :: ClockState
newClock = ClockState 0 0

-- tick returns (new clock state, currently emitted bit value)
tick :: ClockState -> (ClockState, Bit)
tick st = 
    let nv = 1 - clockValue st
        nc = cycleCount st + if nv == 1 then 1 else 0
    in (ClockState nv nc, nv)

data ClockDivider = ClockDivider
    { cdDivisor   :: Int
    , cdCounter   :: Int
    , cdValue     :: Bit
    , cdBaseState :: ClockState
    } deriving (Show, Eq)

newClockDivider :: Int -> ClockDivider
newClockDivider d = ClockDivider d 0 0 newClock

tickDivider :: ClockDivider -> (ClockDivider, Bit)
tickDivider cd =
    let (newBase, baseBit) = tick (cdBaseState cd)
    in if baseBit == 1
       then let newCounter = cdCounter cd + 1
            in if newCounter >= cdDivisor cd
               then let nv = 1 - cdValue cd
                    in (cd { cdCounter = 0, cdValue = nv, cdBaseState = newBase }, nv)
               else (cd { cdCounter = newCounter, cdBaseState = newBase }, cdValue cd)
       else (cd { cdBaseState = newBase }, cdValue cd)

data MultiPhaseClock = MultiPhaseClock
    { mpcPhases    :: Int
    , mpcValue     :: Int -- current active phase index
    , mpcBaseState :: ClockState
    } deriving (Show, Eq)

newMultiPhaseClock :: Int -> MultiPhaseClock
newMultiPhaseClock phases = 
    MultiPhaseClock phases 0 newClock

tickMultiPhase :: MultiPhaseClock -> (MultiPhaseClock, [Bit])
tickMultiPhase mpc =
    let (newBase, baseBit) = tick (mpcBaseState mpc)
    in if baseBit == 1
       then let nv = (mpcValue mpc + 1) `mod` (mpcPhases mpc)
                bits = [if i == nv then 1 else 0 | i <- [0..(mpcPhases mpc - 1)]]
            in (mpc { mpcValue = nv, mpcBaseState = newBase }, bits)
       else let bits = [if i == mpcValue mpc then 1 else 0 | i <- [0..(mpcPhases mpc - 1)]]
            in (mpc { mpcBaseState = newBase }, bits)
