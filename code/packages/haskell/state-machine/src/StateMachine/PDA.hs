module StateMachine.PDA
    ( PushdownAutomaton(..)
    , newPushdownAutomaton
    , processPDA
    , processPDASequence
    , acceptsPDA
    , resetPDA
    ) where

import Control.Monad (foldM, unless, when)
import Data.List (intercalate)
import qualified Data.Map.Strict as Map
import Data.Map.Strict (Map)
import qualified Data.Set as Set
import Data.Set (Set)
import StateMachine.Types

data PushdownAutomaton = PushdownAutomaton
    { pdaStatesSet :: Set State
    , pdaInputAlphabetSet :: Set Event
    , pdaStackAlphabetSet :: Set String
    , pdaTransitionsList :: [PDATransition]
    , pdaInitialStateName :: State
    , pdaInitialStackSymbol :: String
    , pdaAcceptingSet :: Set State
    , pdaTransitionIndex :: Map (State, Maybe Event, String) PDATransition
    , pdaCurrentStateName :: State
    , pdaStackContents :: [String]
    , pdaTraceEntries :: [PDATraceEntry]
    }
    deriving (Eq, Show)

newPushdownAutomaton :: [State] -> [Event] -> [String] -> [PDATransition] -> State -> String -> [State] -> Either String PushdownAutomaton
newPushdownAutomaton states inputAlphabet stackAlphabet transitions initialState initialStackSymbol acceptingStates = do
    let stateSet = Set.fromList states
        inputSet = Set.fromList inputAlphabet
        stackSet = Set.fromList stackAlphabet
        acceptingSet = Set.fromList acceptingStates
        keys = map transitionKey transitions
    unless (not (Set.null stateSet)) (Left "states set must be non-empty")
    unless (Set.member initialState stateSet) $
        Left ("initial state is not in the states set: " ++ initialState)
    unless (Set.member initialStackSymbol stackSet) $
        Left ("initial stack symbol is not in the stack alphabet: " ++ initialStackSymbol)
    unless (acceptingSet `Set.isSubsetOf` stateSet) $
        Left "accepting states must be a subset of the states set"
    when (length keys /= Set.size (Set.fromList keys)) $
        Left "duplicate transitions are not allowed in this deterministic PDA"
    mapM_ (validatePDATransition stateSet inputSet stackSet) transitions
    let index = Map.fromList [(transitionKey transition, transition) | transition <- transitions]
    pure
        PushdownAutomaton
            { pdaStatesSet = stateSet
            , pdaInputAlphabetSet = inputSet
            , pdaStackAlphabetSet = stackSet
            , pdaTransitionsList = transitions
            , pdaInitialStateName = initialState
            , pdaInitialStackSymbol = initialStackSymbol
            , pdaAcceptingSet = acceptingSet
            , pdaTransitionIndex = index
            , pdaCurrentStateName = initialState
            , pdaStackContents = [initialStackSymbol]
            , pdaTraceEntries = []
            }

processPDA :: Event -> PushdownAutomaton -> Either String PushdownAutomaton
processPDA event machine = do
    unless (Set.member event (pdaInputAlphabetSet machine)) $
        Left ("event is not in the input alphabet: " ++ event)
    case lookupTransition (Just event) machine of
        Just transition -> applyTransition transition machine
        Nothing -> do
            machineWithEpsilon <- exhaustEpsilon machine
            applyNamedTransition (Just event) machineWithEpsilon

processPDASequence :: [Event] -> PushdownAutomaton -> Either String PushdownAutomaton
processPDASequence events machine = do
    let startMachine = resetPDA machine
    endMachine <- foldM (\currentMachine event -> processPDA event currentMachine) startMachine events
    exhaustEpsilon endMachine

acceptsPDA :: [Event] -> PushdownAutomaton -> Either String Bool
acceptsPDA events machine = do
    finalMachine <- processPDASequence events machine
    pure (Set.member (pdaCurrentStateName finalMachine) (pdaAcceptingSet finalMachine))

resetPDA :: PushdownAutomaton -> PushdownAutomaton
resetPDA machine =
    machine
        { pdaCurrentStateName = pdaInitialStateName machine
        , pdaStackContents = [pdaInitialStackSymbol machine]
        , pdaTraceEntries = []
        }

exhaustEpsilon :: PushdownAutomaton -> Either String PushdownAutomaton
exhaustEpsilon machine = go Set.empty machine
  where
    go seen currentMachine =
        case lookupTransition Nothing currentMachine of
            Nothing -> Right currentMachine
            Just transition ->
                let signature = (pdaCurrentStateName currentMachine, pdaStackContents currentMachine)
                 in if signature `Set.member` seen
                        then Left "epsilon cycle detected while processing the PDA"
                        else do
                            nextMachine <- applyTransition transition currentMachine
                            go (Set.insert signature seen) nextMachine

applyNamedTransition :: Maybe Event -> PushdownAutomaton -> Either String PushdownAutomaton
applyNamedTransition maybeEvent machine =
    case lookupTransition maybeEvent machine of
        Nothing ->
            Left
                ( "no transition defined for "
                    ++ transitionDescription maybeEvent machine
                )
        Just transition -> applyTransition transition machine

lookupTransition :: Maybe Event -> PushdownAutomaton -> Maybe PDATransition
lookupTransition maybeEvent machine =
    case reverse (pdaStackContents machine) of
        [] -> Nothing
        topSymbol : _ ->
            Map.lookup (pdaCurrentStateName machine, maybeEvent, topSymbol) (pdaTransitionIndex machine)

applyTransition :: PDATransition -> PushdownAutomaton -> Either String PushdownAutomaton
applyTransition transition machine =
    case reverse (pdaStackContents machine) of
        [] -> Left "cannot apply a PDA transition with an empty stack"
        topSymbol : _ -> do
            unless (topSymbol == pdaTransitionStackRead transition) $
                Left "top-of-stack mismatch for PDA transition"
            let stackWithoutTop = init (pdaStackContents machine)
                stackAfter = stackWithoutTop ++ pdaTransitionStackPush transition
                traceEntry =
                    PDATraceEntry
                        { pdaTraceSource = pdaCurrentStateName machine
                        , pdaTraceEvent = pdaTransitionEvent transition
                        , pdaTraceStackRead = pdaTransitionStackRead transition
                        , pdaTraceTarget = pdaTransitionTarget transition
                        , pdaTraceStackPush = pdaTransitionStackPush transition
                        , pdaTraceStackAfter = stackAfter
                        }
             in pure
                    machine
                        { pdaCurrentStateName = pdaTransitionTarget transition
                        , pdaStackContents = stackAfter
                        , pdaTraceEntries = pdaTraceEntries machine ++ [traceEntry]
                        }

transitionKey :: PDATransition -> (State, Maybe Event, String)
transitionKey transition =
    ( pdaTransitionSource transition
    , pdaTransitionEvent transition
    , pdaTransitionStackRead transition
    )

validatePDATransition :: Set State -> Set Event -> Set String -> PDATransition -> Either String ()
validatePDATransition stateSet inputSet stackSet transition = do
    unless (Set.member (pdaTransitionSource transition) stateSet) $
        Left ("transition source is not in the states set: " ++ pdaTransitionSource transition)
    unless (Set.member (pdaTransitionTarget transition) stateSet) $
        Left ("transition target is not in the states set: " ++ pdaTransitionTarget transition)
    case pdaTransitionEvent transition of
        Nothing -> pure ()
        Just symbol ->
            unless (Set.member symbol inputSet) $
                Left ("transition event is not in the input alphabet: " ++ symbol)
    unless (Set.member (pdaTransitionStackRead transition) stackSet) $
        Left ("stack read symbol is not in the stack alphabet: " ++ pdaTransitionStackRead transition)
    mapM_
        (\symbol ->
            unless (Set.member symbol stackSet) $
                Left ("stack push symbol is not in the stack alphabet: " ++ symbol)
        )
        (pdaTransitionStackPush transition)

transitionDescription :: Maybe Event -> PushdownAutomaton -> String
transitionDescription maybeEvent machine =
    "(state="
        ++ pdaCurrentStateName machine
        ++ ", event="
        ++ maybe "epsilon" id maybeEvent
        ++ ", stack="
        ++ intercalate "," (pdaStackContents machine)
        ++ ")"
