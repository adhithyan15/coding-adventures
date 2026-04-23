module LsmTree
    ( LsmTree
    , new
    , put
    , delete
    , get
    , contains
    , flush
    , compact
    , rangeQuery
    , size
    ) where

import qualified Data.Map.Strict as Map
import Data.Map.Strict (Map)

data LsmTree k v = LsmTree
    { memtableLimit :: Int
    , memtable :: Map k (Maybe v)
    , segments :: [Map k (Maybe v)]
    }
    deriving (Eq, Show)

new :: Int -> LsmTree k v
new threshold =
    LsmTree
        { memtableLimit = max 1 threshold
        , memtable = Map.empty
        , segments = []
        }

put :: Ord k => k -> v -> LsmTree k v -> LsmTree k v
put key value treeValue =
    autoFlush $
        treeValue{memtable = Map.insert key (Just value) (memtable treeValue)}

delete :: Ord k => k -> LsmTree k v -> LsmTree k v
delete key treeValue =
    autoFlush $
        treeValue{memtable = Map.insert key Nothing (memtable treeValue)}

get :: Ord k => k -> LsmTree k v -> Maybe v
get key treeValue =
    case Map.lookup key (memtable treeValue) of
        Just value -> value
        Nothing -> lookupSegments key (segments treeValue)

contains :: Ord k => k -> LsmTree k v -> Bool
contains key = maybe False (const True) . get key

flush :: LsmTree k v -> LsmTree k v
flush treeValue
    | Map.null (memtable treeValue) = treeValue
    | otherwise =
        treeValue
            { memtable = Map.empty
            , segments = memtable treeValue : segments treeValue
            }

compact :: Ord k => LsmTree k v -> LsmTree k v
compact treeValue =
    let merged = foldr Map.union Map.empty (segments treeValue)
        compacted = Map.filter maybePresent merged
     in treeValue{segments = [compacted]}
  where
    maybePresent Nothing = False
    maybePresent (Just _) = True

rangeQuery :: Ord k => k -> k -> LsmTree k v -> [(k, v)]
rangeQuery lower upper treeValue =
    [ (key, value)
    | (key, value) <- Map.toAscList (materialize treeValue)
    , key >= lower
    , key <= upper
    ]

size :: Ord k => LsmTree k v -> Int
size = Map.size . materialize

autoFlush :: LsmTree k v -> LsmTree k v
autoFlush treeValue
    | Map.size (memtable treeValue) >= memtableLimit treeValue = flush treeValue
    | otherwise = treeValue

lookupSegments :: Ord k => k -> [Map k (Maybe v)] -> Maybe v
lookupSegments _ [] = Nothing
lookupSegments key (segment : rest) =
    case Map.lookup key segment of
        Just value -> value
        Nothing -> lookupSegments key rest

materialize :: Ord k => LsmTree k v -> Map k v
materialize treeValue =
    Map.mapMaybe id (Map.union (memtable treeValue) (foldr Map.union Map.empty (segments treeValue)))
