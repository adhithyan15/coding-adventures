module Electronics.Components
    ( Component(..)
    , newResistor
    , newCapacitor
    ) where

data Component = Resistor Double
               | Capacitor Double
               deriving (Show, Eq)

newResistor :: Double -> Component
newResistor ohms = Resistor ohms

newCapacitor :: Double -> Component
newCapacitor farads = Capacitor farads
