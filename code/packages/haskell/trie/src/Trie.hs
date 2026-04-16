module Trie
    ( Trie
    , empty
    , insert
    , lookupValue
    , delete
    , startsWith
    , wordsWithPrefix
    , longestPrefixMatch
    , keys
    , toList
    ) where

import qualified Data.Map.Strict as Map
import Data.Map.Strict (Map)
import Data.List (sort)

data Trie v = Trie
    { trieValue :: Maybe v
    , trieChildren :: Map Char (Trie v)
    }
    deriving (Eq, Show)

empty :: Trie v
empty = Trie Nothing Map.empty

insert :: String -> v -> Trie v -> Trie v
insert [] value trieValue' = trieValue'{trieValue = Just value}
insert (charValue : rest) value trieValue' =
    trieValue'
        { trieChildren =
            Map.alter
                (Just . insert rest value . maybe empty id)
                charValue
                (trieChildren trieValue')
        }

lookupValue :: String -> Trie v -> Maybe v
lookupValue [] trieValue' = trieValue trieValue'
lookupValue (charValue : rest) trieValue' =
    Map.lookup charValue (trieChildren trieValue') >>= lookupValue rest

delete :: String -> Trie v -> Trie v
delete [] trieValue' = trieValue'{trieValue = Nothing}
delete (charValue : rest) trieValue' =
    trieValue'
        { trieChildren =
            Map.update
                (\child ->
                    let updated = delete rest child
                     in if isEmpty updated then Nothing else Just updated
                )
                charValue
                (trieChildren trieValue')
        }

startsWith :: String -> Trie v -> Bool
startsWith prefix trieValue' =
    case descend prefix trieValue' of
        Nothing -> False
        Just _ -> True

wordsWithPrefix :: String -> Trie v -> [(String, v)]
wordsWithPrefix prefix trieValue' =
    case descend prefix trieValue' of
        Nothing -> []
        Just subtreeTrie ->
            [ (prefix ++ suffix, value)
            | (suffix, value) <- collectPairs subtreeTrie
            ]

longestPrefixMatch :: String -> Trie v -> Maybe (String, v)
longestPrefixMatch input trieValue' =
    go "" input trieValue' Nothing
  where
    go current [] currentTrie best =
        case trieValue currentTrie of
            Nothing -> best
            Just value -> Just (current, value)
    go current remaining currentTrie best =
        let best' =
                case trieValue currentTrie of
                    Nothing -> best
                    Just value -> Just (current, value)
         in case remaining of
                [] -> best'
                charValue : rest ->
                    case Map.lookup charValue (trieChildren currentTrie) of
                        Nothing -> best'
                        Just nextTrie -> go (current ++ [charValue]) rest nextTrie best'

keys :: Trie v -> [String]
keys trieValue' = sort (map fst (collectPairs trieValue'))

toList :: Trie v -> [(String, v)]
toList trieValue' =
    [ pair
    | key <- sort (map fst (collectPairs trieValue'))
    , pair <- take 1 (filter ((== key) . fst) (collectPairs trieValue'))
    ]

descend :: String -> Trie v -> Maybe (Trie v)
descend [] trieValue' = Just trieValue'
descend (charValue : rest) trieValue' =
    Map.lookup charValue (trieChildren trieValue') >>= descend rest

collectPairs :: Trie v -> [(String, v)]
collectPairs trieValue' =
    prefixValue ++ childValues
  where
    prefixValue =
        case trieValue trieValue' of
            Nothing -> []
            Just value -> [("", value)]
    childValues =
        concat
            [ [ (charValue : suffix, value)
              | (suffix, value) <- collectPairs child
              ]
            | (charValue, child) <- Map.toList (trieChildren trieValue')
            ]

isEmpty :: Trie v -> Bool
isEmpty trieValue' =
    case trieValue trieValue' of
        Nothing -> Map.null (trieChildren trieValue')
        Just _ -> False
