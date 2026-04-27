module PowerSupply
    ( PowerSupply(..)
    , newPowerSupply
    , readVoltage
    ) where

data PowerSupply = PowerSupply
    { psVoltage :: Double
    } deriving (Show, Eq)

newPowerSupply :: Double -> PowerSupply
newPowerSupply = PowerSupply

readVoltage :: PowerSupply -> Double
readVoltage = psVoltage
