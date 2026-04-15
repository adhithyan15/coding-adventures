module Electronics.Circuit
    ( Node(..)
    , RCFilter(..)
    , newRCFilter
    , stepFilter
    ) where

import Electronics.Components

data Node = Node
    { nodeVoltage :: Double
    } deriving (Show, Eq)

data RCFilter = RCFilter
    { rcResistor  :: Component
    , rcCapacitor :: Component
    , rcNode      :: Node
    } deriving (Show, Eq)

newRCFilter :: Double -> Double -> RCFilter
newRCFilter r c = RCFilter (newResistor r) (newCapacitor c) (Node 0.0)

stepFilter :: RCFilter -> Double -> Double -> RCFilter
stepFilter rc@(RCFilter (Resistor r) (Capacitor c) (Node vOut)) vIn dt =
    let alpha = dt / (r * c)
        newV = vOut + alpha * (vIn - vOut)
    in rc { rcNode = Node newV }
stepFilter rc _ _ = rc
