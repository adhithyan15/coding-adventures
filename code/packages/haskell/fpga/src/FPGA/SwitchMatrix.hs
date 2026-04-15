module FPGA.SwitchMatrix
    ( SwitchMatrix(..)
    , newSwitchMatrix
    , connect
    , disconnect
    , route
    ) where

import LogicGates.Basic
import qualified Data.Set as Set
import qualified Data.Map as Map

data SwitchMatrix = SwitchMatrix
    { smPorts       :: Set.Set String
    , smConnections :: Map.Map String String
    } deriving (Show, Eq)

newSwitchMatrix :: Set.Set String -> Either String SwitchMatrix
newSwitchMatrix ports = do
    if Set.null ports then Left "ports must be non-empty"
    else Right $ SwitchMatrix ports Map.empty

connect :: SwitchMatrix -> String -> String -> Either String SwitchMatrix
connect sm src dest = do
    if not (Set.member src (smPorts sm)) then Left "unknown source"
    else if not (Set.member dest (smPorts sm)) then Left "unknown dest"
    else if src == dest then Left "cannot connect to itself"
    else if Map.member dest (smConnections sm) then Left "already connected"
    else Right $ sm { smConnections = Map.insert dest src (smConnections sm) }

disconnect :: SwitchMatrix -> String -> Either String SwitchMatrix
disconnect sm dest = do
    if not (Set.member dest (smPorts sm)) then Left "unknown port"
    else if not (Map.member dest (smConnections sm)) then Left "not connected"
    else Right $ sm { smConnections = Map.delete dest (smConnections sm) }

route :: SwitchMatrix -> Map.Map String Bit -> Map.Map String Bit
route sm inputs = 
    Map.foldrWithKey (\dest src acc -> case Map.lookup src inputs of
                                        Just v -> Map.insert dest v acc
                                        Nothing -> acc
                     ) Map.empty (smConnections sm)
