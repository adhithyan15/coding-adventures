module FPGA.Bitstream where

import LogicGates.Basic

data SliceConfig = SliceConfig
    { confLutA :: [Bit]
    , confLutB :: [Bit]
    , confFfAEn :: Bool
    , confFfBEn :: Bool
    , confCarryEn :: Bool
    } deriving (Show, Eq)

data CLBConfig = CLBConfig
    { confSlice0 :: SliceConfig
    , confSlice1 :: SliceConfig
    } deriving (Show, Eq)

data RouteConfig = RouteConfig
    { rcSource :: String
    , rcDest   :: String
    } deriving (Show, Eq)

data IOConfig = IOConfig
    { confIOMode :: String
    } deriving (Show, Eq)

data Bitstream = Bitstream
    { bsClbs    :: [(String, CLBConfig)]
    , bsRouting :: [(String, [RouteConfig])]
    , bsIo      :: [(String, IOConfig)]
    , bsLutK    :: Int
    } deriving (Show, Eq)

emptyBitstream :: Bitstream
emptyBitstream = Bitstream [] [] [] 4
