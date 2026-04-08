module FPGA.Fabric
    ( Fabric(..)
    , newFabric
    , configureFabric
    ) where

import LogicGates.Basic
import FPGA.Types
import FPGA.CLB
import FPGA.SwitchMatrix
import FPGA.IOBlock
import FPGA.Bitstream
import qualified Data.Map as Map

data Fabric = Fabric
    { fbRows     :: Int
    , fbCols     :: Int
    , fbLutK     :: Int
    , fbClbs     :: Map.Map String CLB
    , fbSwitches :: Map.Map String SwitchMatrix
    , fbIOBlocks :: Map.Map String IOBlock
    } deriving (Show, Eq)

newFabric :: Int -> Int -> Int -> Either String Fabric
newFabric rows cols lutK = do
    if rows < 1 || cols < 1 then Left "Dimensions must be > 0"
    else if lutK < 2 || lutK > 6 then Left "LUT K must be 2..6"
    else Right $ Fabric rows cols lutK Map.empty Map.empty Map.empty

configureFabric :: Fabric -> Bitstream -> Either String Fabric
configureFabric fb bs = do
    -- simplified loading mock for educational purposes
    Right fb
