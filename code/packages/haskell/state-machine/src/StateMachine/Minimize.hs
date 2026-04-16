module StateMachine.Minimize
    ( minimizeDFA
    ) where

import Data.List (find, intercalate, sort, sortBy)
import qualified Data.Map.Strict as Map
import Data.Map.Strict (Map)
import qualified Data.Set as Set
import Data.Set (Set)
import StateMachine.DFA
import StateMachine.Types

minimizeDFA :: DFA -> Either String DFA
minimizeDFA machine =
    newDFA minimizedStates alphabetList minimizedTransitions minimizedInitial minimizedAccepting
  where
    reachable = Set.fromList (reachableStates machine)
    alphabetList = sort (Set.toList (dfaAlphabetSet machine))
    accepting = Set.intersection reachable (dfaAcceptingSet machine)
    nonAccepting = Set.difference reachable accepting
    initialPartitions =
        filter (not . Set.null) [accepting, nonAccepting]
    partitions = refine initialPartitions
    minimizedStates = map partitionName partitions
    minimizedAccepting =
        [ partitionName partition
        | partition <- partitions
        , not (Set.null (Set.intersection partition accepting))
        ]
    minimizedInitial =
        maybe
            (dfaInitialState machine)
            partitionName
            (find (\partition -> dfaInitialState machine `Set.member` partition) partitions)
    minimizedTransitions =
        [ ((partitionName partition, event), partitionName targetPartition)
        | partition <- partitions
        , representative <- firstState partition
        , event <- alphabetList
        , Just target <- [Map.lookup (representative, event) (dfaTransitionsMap machine)]
        , Just targetPartition <- [find (Set.member target) partitions]
        ]

    refine partitions0 =
        let updated = concatMap (splitPartition partitions0) partitions0
         in if updated == partitions0 then partitions0 else refine updated

    splitPartition allPartitions partition
        | Set.size partition <= 1 = [partition]
        | otherwise =
            case firstUsefulSplit of
                Nothing -> [partition]
                Just pieces -> pieces
      where
        partitionIndexMap =
            Map.fromList
                [ (state, index)
                | (index, candidatePartition) <- zip [0 ..] allPartitions
                , state <- Set.toList candidatePartition
                ]
        signatureFor event state =
            fmap (`Map.lookup` partitionIndexMap) (Map.lookup (state, event) (dfaTransitionsMap machine))
        splitFor event =
            Map.elems
                ( Map.fromListWith
                    Set.union
                    [ (signatureFor event state, Set.singleton state)
                    | state <- Set.toList partition
                    ]
                )
        firstUsefulSplit =
            find (\pieces -> length pieces > 1) [sortSets (splitFor event) | event <- alphabetList]

sortSets :: [Set State] -> [Set State]
sortSets =
    sortOnSet
        . filter (not . Set.null)
  where
    sortOnSet = sortOn (\stateSet -> sort (Set.toList stateSet))

partitionName :: Set State -> String
partitionName partition =
    case sort (Set.toList partition) of
        [single] -> single
        names -> "{" ++ intercalate "," names ++ "}"

firstState :: Set State -> [State]
firstState stateSet =
    case sort (Set.toList stateSet) of
        [] -> []
        firstValue : _ -> [firstValue]

sortOn :: Ord b => (a -> b) -> [a] -> [a]
sortOn f = sortBy (\left right -> compare (f left) (f right))
