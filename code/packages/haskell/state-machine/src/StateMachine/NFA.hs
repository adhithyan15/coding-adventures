module StateMachine.NFA
    ( NFA(..)
    , newNFA
    , epsilonClosure
    , processNFA
    , acceptsNFA
    , resetNFA
    , currentNFAStates
    , nfaToDFA
    ) where

import Control.Monad (foldM, unless, when)
import Data.List (intercalate, sort)
import qualified Data.Map.Strict as Map
import Data.Map.Strict (Map)
import qualified Data.Set as Set
import Data.Set (Set)
import qualified DirectedGraph as DG
import StateMachine.DFA
import StateMachine.Types

data NFA = NFA
    { nfaStatesSet :: Set State
    , nfaAlphabetSet :: Set Event
    , nfaTransitionsMap :: Map (State, Maybe Event) (Set State)
    , nfaInitialState :: State
    , nfaAcceptingSet :: Set State
    , nfaGraph :: DG.DirectedGraph
    , nfaCurrentSet :: Set State
    }
    deriving (Eq, Show)

newNFA :: [State] -> [Event] -> [((State, Maybe Event), [State])] -> State -> [State] -> Either String NFA
newNFA states alphabet transitions initialState acceptingStates = do
    let stateSet = Set.fromList states
        alphabetSet = Set.fromList alphabet
        acceptingSet = Set.fromList acceptingStates
    unless (not (Set.null stateSet)) (Left "states set must be non-empty")
    when (Set.member epsilon alphabetSet) $
        Left "alphabet must not contain the empty string; use Nothing for epsilon transitions"
    unless (Set.member initialState stateSet) $
        Left ("initial state is not in the states set: " ++ initialState)
    unless (acceptingSet `Set.isSubsetOf` stateSet) $
        Left "accepting states must be a subset of the states set"
    mapM_ (validateNFATransition stateSet alphabetSet) transitions
    let transitionMap =
            Map.fromListWith Set.union
                [ (key, Set.fromList targets)
                | (key, targets) <- transitions
                ]
        graph =
            foldl
                (\acc ((source, _), targets) -> foldl (\g target -> DG.addEdge source target g) acc targets)
                (foldl (flip DG.addNode) DG.empty (Set.toList stateSet))
                transitions
        machine0 =
            NFA
                { nfaStatesSet = stateSet
                , nfaAlphabetSet = alphabetSet
                , nfaTransitionsMap = transitionMap
                , nfaInitialState = initialState
                , nfaAcceptingSet = acceptingSet
                , nfaGraph = graph
                , nfaCurrentSet = Set.empty
                }
    pure (resetNFA machine0)

epsilonClosure :: NFA -> Set State -> Set State
epsilonClosure machine startStates = go startStates (Set.toList startStates)
  where
    go closed [] = closed
    go closed (state : rest) =
        let next = Map.findWithDefault Set.empty (state, Nothing) (nfaTransitionsMap machine)
            unseen = Set.toList (Set.difference next closed)
         in go (Set.union closed next) (unseen ++ rest)

processNFA :: Event -> NFA -> Either String NFA
processNFA event machine = do
    unless (Set.member event (nfaAlphabetSet machine)) $
        Left ("event is not in the alphabet: " ++ event)
    let nextStates =
            Set.unions
                [ Map.findWithDefault Set.empty (state, Just event) (nfaTransitionsMap machine)
                | state <- Set.toList (nfaCurrentSet machine)
                ]
    pure machine {nfaCurrentSet = epsilonClosure machine nextStates}

acceptsNFA :: [Event] -> NFA -> Either String Bool
acceptsNFA events machine = do
    finalMachine <- foldM (\currentMachine event -> processNFA event currentMachine) (resetNFA machine) events
    pure (not (Set.null (Set.intersection (nfaCurrentSet finalMachine) (nfaAcceptingSet finalMachine))))

resetNFA :: NFA -> NFA
resetNFA machine =
    machine
        { nfaCurrentSet =
            epsilonClosure machine (Set.singleton (nfaInitialState machine))
        }

currentNFAStates :: NFA -> [State]
currentNFAStates = sort . Set.toList . nfaCurrentSet

nfaToDFA :: NFA -> Either String DFA
nfaToDFA machine =
    newDFA dfaStates alphabetList dfaTransitions dfaInitial acceptingStates
  where
    alphabetList = sort (Set.toList (nfaAlphabetSet machine))
    initialSet = epsilonClosure machine (Set.singleton (nfaInitialState machine))
    visitedSets = explore Set.empty [initialSet]
    dfaStates = map stateSetName visitedSets
    dfaInitial = stateSetName initialSet
    acceptingStates =
        [ stateSetName stateSet
        | stateSet <- visitedSets
        , not (Set.null (Set.intersection stateSet (nfaAcceptingSet machine)))
        ]
    dfaTransitions =
        [ ((stateSetName stateSet, event), stateSetName nextStateSet)
        | stateSet <- visitedSets
        , event <- alphabetList
        , let nextStateSet = nextSet stateSet event
        , not (Set.null nextStateSet)
        ]

    nextSet stateSet event =
        epsilonClosure
            machine
            ( Set.unions
                [ Map.findWithDefault Set.empty (state, Just event) (nfaTransitionsMap machine)
                | state <- Set.toList stateSet
                ]
            )

    explore _ [] = []
    explore seen (stateSet : rest)
        | stateSet `Set.member` seen = explore seen rest
        | otherwise =
            let nextSets =
                    [ nextSet stateSet event
                    | event <- alphabetList
                    , let candidate = nextSet stateSet event
                    , not (Set.null candidate)
                    ]
             in stateSet : explore (Set.insert stateSet seen) (rest ++ nextSets)

stateSetName :: Set State -> String
stateSetName stateSet =
    case sort (Set.toList stateSet) of
        [] -> "{}"
        [single] -> single
        names -> "{" ++ intercalate "," names ++ "}"

validateNFATransition :: Set State -> Set Event -> ((State, Maybe Event), [State]) -> Either String ()
validateNFATransition stateSet alphabetSet ((source, event), targets) = do
    unless (Set.member source stateSet) $
        Left ("transition source is not in the states set: " ++ source)
    case event of
        Nothing -> pure ()
        Just symbol ->
            unless (Set.member symbol alphabetSet) $
                Left ("transition event is not in the alphabet: " ++ symbol)
    mapM_
        (\target ->
            unless (Set.member target stateSet) $
                Left ("transition target is not in the states set: " ++ target)
        )
        targets
