module RadixTree
    ( description
    , RadixTree
    , empty
    , insert
    , lookup
    , contains
    , delete
    , keys
    , keysWithPrefix
    , fromList
    ) where

import Prelude hiding (lookup)

import qualified Data.ByteString as BS
import qualified Data.Map.Strict as Map
import Data.Maybe (isJust)
import Data.Word (Word8)

description :: String
description = "Haskell radix tree for keyspace indexing and prefix matching"

data RadixTree value = RadixTree
    { radixValue :: Maybe value
    , radixChildren :: Map.Map Word8 (RadixTree value)
    }
    deriving (Eq, Show)

empty :: RadixTree value
empty = RadixTree Nothing Map.empty

insert :: BS.ByteString -> value -> RadixTree value -> RadixTree value
insert keyBytes value = go (BS.unpack keyBytes)
  where
    go [] node = node {radixValue = Just value}
    go (byte : rest) node =
        let child = Map.findWithDefault empty byte (radixChildren node)
            updatedChild = go rest child
         in node
                { radixChildren =
                    Map.insert byte updatedChild (radixChildren node)
                }

lookup :: BS.ByteString -> RadixTree value -> Maybe value
lookup keyBytes = go (BS.unpack keyBytes)
  where
    go [] node = radixValue node
    go (byte : rest) node =
        case Map.lookup byte (radixChildren node) of
            Nothing -> Nothing
            Just child -> go rest child

contains :: BS.ByteString -> RadixTree value -> Bool
contains keyBytes = isJust . lookup keyBytes

delete :: BS.ByteString -> RadixTree value -> RadixTree value
delete keyBytes = prune . go (BS.unpack keyBytes)
  where
    go [] node = node {radixValue = Nothing}
    go (byte : rest) node =
        case Map.lookup byte (radixChildren node) of
            Nothing -> node
            Just child ->
                let updatedChild = prune (go rest child)
                    updatedChildren =
                        if isNodeEmpty updatedChild
                            then Map.delete byte (radixChildren node)
                            else Map.insert byte updatedChild (radixChildren node)
                 in node {radixChildren = updatedChildren}

keys :: RadixTree value -> [BS.ByteString]
keys = map BS.pack . collectRaw []

keysWithPrefix :: BS.ByteString -> RadixTree value -> [BS.ByteString]
keysWithPrefix prefixBytes tree =
    case descend (BS.unpack prefixBytes) tree of
        Nothing -> []
        Just subtree ->
            map
                (BS.pack . (BS.unpack prefixBytes ++))
                (collectRaw [] subtree)

fromList :: [(BS.ByteString, value)] -> RadixTree value
fromList = foldr (uncurry insert) empty

descend :: [Word8] -> RadixTree value -> Maybe (RadixTree value)
descend prefixBytes tree =
    case prefixBytes of
        [] -> Just tree
        byte : rest ->
            Map.lookup byte (radixChildren tree) >>= descend rest

collectRaw :: [Word8] -> RadixTree value -> [[Word8]]
collectRaw prefixBytes tree =
    currentValue ++ childValues
  where
    currentValue =
        case radixValue tree of
            Nothing -> []
            Just _ -> [prefixBytes]
    childValues =
        concatMap
            (\(byte, child) -> collectRaw (prefixBytes ++ [byte]) child)
            (Map.toAscList (radixChildren tree))

prune :: RadixTree value -> RadixTree value
prune node =
    node
        { radixChildren =
            Map.filter (not . isNodeEmpty) (radixChildren node)
        }

isNodeEmpty :: RadixTree value -> Bool
isNodeEmpty node =
    case radixValue node of
        Just _ -> False
        Nothing -> Map.null (radixChildren node)
