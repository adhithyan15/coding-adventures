module StateMachine.Types
    ( State
    , Event
    , epsilon
    , TransitionRecord(..)
    , ModeTransitionRecord(..)
    , PDATransition(..)
    , PDATraceEntry(..)
    ) where

type State = String

type Event = String

epsilon :: Event
epsilon = ""

data TransitionRecord = TransitionRecord
    { transitionSource :: State
    , transitionEvent :: Maybe Event
    , transitionTarget :: State
    , transitionActionName :: Maybe String
    }
    deriving (Eq, Show)

data ModeTransitionRecord = ModeTransitionRecord
    { modeTransitionFrom :: String
    , modeTransitionTrigger :: Event
    , modeTransitionTo :: String
    }
    deriving (Eq, Show)

data PDATransition = PDATransition
    { pdaTransitionSource :: State
    , pdaTransitionEvent :: Maybe Event
    , pdaTransitionStackRead :: String
    , pdaTransitionTarget :: State
    , pdaTransitionStackPush :: [String]
    }
    deriving (Eq, Show)

data PDATraceEntry = PDATraceEntry
    { pdaTraceSource :: State
    , pdaTraceEvent :: Maybe Event
    , pdaTraceStackRead :: String
    , pdaTraceTarget :: State
    , pdaTraceStackPush :: [String]
    , pdaTraceStackAfter :: [String]
    }
    deriving (Eq, Show)
