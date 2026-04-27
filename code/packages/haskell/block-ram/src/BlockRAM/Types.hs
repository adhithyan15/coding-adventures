module BlockRAM.Types
    ( ReadMode(..)
    ) where

data ReadMode = ReadFirst | WriteFirst | NoChange deriving (Show, Eq)
