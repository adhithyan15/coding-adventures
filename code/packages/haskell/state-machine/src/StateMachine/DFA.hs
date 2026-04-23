module StateMachine.DFA
    ( DFA(..)
    , newDFA
    , processDFA
    , processDFASequence
    , acceptsDFA
    , resetDFA
    , reachableStates
    , isCompleteDFA
    , validateDFA
    ) where

import Control.Monad (foldM, unless, when)
import Data.List (sort)
import qualified Data.Map.Strict as Map
import Data.Map.Strict (Map)
import qualified Data.Set as Set
import Data.Set (Set)
import qualified DirectedGraph as DG
import StateMachine.Types

data DFA = DFA
    { dfaStatesSet :: Set State
    , dfaAlphabetSet :: Set Event
    , dfaTransitionsMap :: Map (State, Event) State
    , dfaInitialState :: State
    , dfaAcceptingSet :: Set State
    , dfaGraph :: DG.DirectedGraph
    , dfaCurrentState :: State
    , dfaTraceRecords :: [TransitionRecord]
    }
    deriving (Eq, Show)

newDFA :: [State] -> [Event] -> [((State, Event), State)] -> State -> [State] -> Either String DFA
newDFA states alphabet transitions initialState acceptingStates = do
    let stateSet = Set.fromList states
        alphabetSet = Set.fromList alphabet
        acceptingSet = Set.fromList acceptingStates
        transitionKeys = Set.fromList (map fst transitions)
        transitionMap = Map.fromList transitions
    unless (not (Set.null stateSet)) (Left "states set must be non-empty")
    unless (not (Set.null alphabetSet)) (Left "alphabet must be non-empty")
    when (Set.size transitionKeys /= length transitions) $
        Left "duplicate transitions are not allowed in a DFA"
    unless (Set.member initialState stateSet) $
        Left ("initial state is not in the states set: " ++ initialState)
    unless (acceptingSet `Set.isSubsetOf` stateSet) $
        Left "accepting states must be a subset of the states set"
    mapM_ (validateTransition stateSet alphabetSet) transitions
    let graph =
            foldl
                (\acc ((source, _), target) -> DG.addEdge source target acc)
                (foldl (flip DG.addNode) DG.empty (Set.toList stateSet))
                transitions
    pure
        DFA
            { dfaStatesSet = stateSet
            , dfaAlphabetSet = alphabetSet
            , dfaTransitionsMap = transitionMap
            , dfaInitialState = initialState
            , dfaAcceptingSet = acceptingSet
            , dfaGraph = graph
            , dfaCurrentState = initialState
            , dfaTraceRecords = []
            }

processDFA :: Event -> DFA -> Either String DFA
processDFA event machine = do
    unless (Set.member event (dfaAlphabetSet machine)) $
        Left ("event is not in the alphabet: " ++ event)
    let source = dfaCurrentState machine
    target <-
        maybe
            (Left ("no transition defined for (" ++ source ++ ", " ++ event ++ ")"))
            Right
            (Map.lookup (source, event) (dfaTransitionsMap machine))
    pure
        machine
            { dfaCurrentState = target
            , dfaTraceRecords =
                dfaTraceRecords machine
                    ++ [ TransitionRecord
                            { transitionSource = source
                            , transitionEvent = Just event
                            , transitionTarget = target
                            , transitionActionName = Nothing
                            }
                       ]
            }

processDFASequence :: [Event] -> DFA -> Either String DFA
processDFASequence events machine =
    foldM (\currentMachine event -> processDFA event currentMachine) machine events

acceptsDFA :: [Event] -> DFA -> Either String Bool
acceptsDFA events machine = do
    finalMachine <- processDFASequence events (resetDFA machine)
    pure (Set.member (dfaCurrentState finalMachine) (dfaAcceptingSet finalMachine))

resetDFA :: DFA -> DFA
resetDFA machine =
    machine
        { dfaCurrentState = dfaInitialState machine
        , dfaTraceRecords = []
        }

reachableStates :: DFA -> [State]
reachableStates machine =
    sort
        . Set.toList
        $ Set.insert
            (dfaInitialState machine)
            (Set.fromList (DG.transitiveDependents (dfaInitialState machine) (dfaGraph machine)))

isCompleteDFA :: DFA -> Bool
isCompleteDFA machine =
    all
        (\(state, event) -> Map.member (state, event) (dfaTransitionsMap machine))
        [ (state, event)
        | state <- Set.toList (dfaStatesSet machine)
        , event <- Set.toList (dfaAlphabetSet machine)
        ]

validateDFA :: DFA -> [String]
validateDFA machine =
    unreachableWarnings ++ incompletenessWarnings
  where
    reachable = Set.fromList (reachableStates machine)
    unreachable = sort . Set.toList $ Set.difference (dfaStatesSet machine) reachable
    unreachableWarnings =
        [ "unreachable state: " ++ state
        | state <- unreachable
        ]
    incompletenessWarnings =
        [ "missing transition for (" ++ state ++ ", " ++ event ++ ")"
        | state <- sort (Set.toList (dfaStatesSet machine))
        , event <- sort (Set.toList (dfaAlphabetSet machine))
        , Map.notMember (state, event) (dfaTransitionsMap machine)
        ]

validateTransition :: Set State -> Set Event -> ((State, Event), State) -> Either String ()
validateTransition stateSet alphabetSet ((source, event), target) = do
    unless (Set.member source stateSet) $
        Left ("transition source is not in the states set: " ++ source)
    unless (Set.member event alphabetSet) $
        Left ("transition event is not in the alphabet: " ++ event)
    unless (Set.member target stateSet) $
        Left ("transition target is not in the states set: " ++ target)
