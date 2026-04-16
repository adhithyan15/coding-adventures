module StateMachine.Modal
    ( ModalStateMachine(..)
    , newModalStateMachine
    , switchMode
    , processModal
    , resetModal
    , activeMachine
    ) where

import Control.Monad (unless)
import qualified Data.Map.Strict as Map
import Data.Map.Strict (Map)
import qualified DirectedGraph as DG
import StateMachine.DFA
import StateMachine.Types

data ModalStateMachine = ModalStateMachine
    { modalModes :: Map String DFA
    , modalTransitions :: Map (String, Event) String
    , modalInitialMode :: String
    , modalModeGraph :: DG.DirectedGraph
    , modalCurrentMode :: String
    , modalTraceRecords :: [ModeTransitionRecord]
    }
    deriving (Eq, Show)

newModalStateMachine :: [(String, DFA)] -> [((String, Event), String)] -> String -> Either String ModalStateMachine
newModalStateMachine modes transitions initialMode = do
    let modeMap = Map.fromList modes
        transitionMap = Map.fromList transitions
        graph =
            foldl
                (\acc ((source, _), target) -> DG.addEdge source target acc)
                (foldl (flip DG.addNode) DG.empty (map fst modes))
                transitions
    unless (not (Map.null modeMap)) (Left "at least one mode must be provided")
    unless (Map.member initialMode modeMap) $
        Left ("initial mode is not in the modes map: " ++ initialMode)
    mapM_
        (\((source, _), target) -> do
            unless (Map.member source modeMap) $
                Left ("mode transition source is not valid: " ++ source)
            unless (Map.member target modeMap) $
                Left ("mode transition target is not valid: " ++ target)
        )
        transitions
    pure
        ModalStateMachine
            { modalModes = modeMap
            , modalTransitions = transitionMap
            , modalInitialMode = initialMode
            , modalModeGraph = graph
            , modalCurrentMode = initialMode
            , modalTraceRecords = []
            }

activeMachine :: ModalStateMachine -> Maybe DFA
activeMachine machine = Map.lookup (modalCurrentMode machine) (modalModes machine)

switchMode :: Event -> ModalStateMachine -> Either String ModalStateMachine
switchMode trigger machine = do
    targetMode <-
        maybe
            (Left ("no mode transition defined for (" ++ modalCurrentMode machine ++ ", " ++ trigger ++ ")"))
            Right
            (Map.lookup (modalCurrentMode machine, trigger) (modalTransitions machine))
    unless (Map.member targetMode (modalModes machine)) $
        Left ("target mode is not available: " ++ targetMode)
    let modes =
            Map.adjust resetDFA targetMode (modalModes machine)
     in pure
            machine
                { modalModes = modes
                , modalCurrentMode = targetMode
                , modalTraceRecords =
                    modalTraceRecords machine
                        ++ [ ModeTransitionRecord
                                { modeTransitionFrom = modalCurrentMode machine
                                , modeTransitionTrigger = trigger
                                , modeTransitionTo = targetMode
                                }
                           ]
                }

processModal :: Event -> ModalStateMachine -> Either String ModalStateMachine
processModal event machine = do
    currentMachine <-
        maybe
            (Left ("no active machine for mode: " ++ modalCurrentMode machine))
            Right
            (activeMachine machine)
    updatedMachine <- processDFA event currentMachine
    pure
        machine
            { modalModes = Map.insert (modalCurrentMode machine) updatedMachine (modalModes machine)
            }

resetModal :: ModalStateMachine -> ModalStateMachine
resetModal machine =
    machine
        { modalModes = Map.map resetDFA (modalModes machine)
        , modalCurrentMode = modalInitialMode machine
        , modalTraceRecords = []
        }
