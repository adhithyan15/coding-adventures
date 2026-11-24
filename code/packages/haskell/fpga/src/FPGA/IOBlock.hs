module FPGA.IOBlock
    ( IOMode(..)
    , IOBlock(..)
    , newIOBlock
    , drivePad
    , driveInternal
    , readInternal
    , readPad
    ) where

import LogicGates

data IOMode = InputMode | OutputMode | TristateMode deriving (Show, Eq)

data IOBlock = IOBlock
    { ioName          :: String
    , ioMode          :: IOMode
    , ioPadValue      :: Bit
    , ioInternalValue :: Bit
    } deriving (Show, Eq)

newIOBlock :: String -> IOMode -> Either LogicGateError IOBlock
newIOBlock name mode = do
    if null name then Left (InvalidShape "name must be non-empty")
    else Right $ IOBlock name mode 0 0

drivePad :: IOBlock -> Bit -> Either LogicGateError IOBlock
drivePad io val = do
    _ <- validateBit "val" val
    Right $ io { ioPadValue = val }

driveInternal :: IOBlock -> Bit -> Either LogicGateError IOBlock
driveInternal io val = do
    _ <- validateBit "val" val
    Right $ io { ioInternalValue = val }

readInternal :: IOBlock -> Bit
readInternal io = 
    if ioMode io == InputMode then ioPadValue io else ioInternalValue io

readPad :: IOBlock -> Either LogicGateError (Maybe Bit)
readPad io = do
    if ioMode io == InputMode then Right (Just (ioPadValue io))
    else if ioMode io == TristateMode then triState (ioInternalValue io) 0
    else triState (ioInternalValue io) 1
