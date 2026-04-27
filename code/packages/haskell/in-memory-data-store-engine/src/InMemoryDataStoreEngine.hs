module InMemoryDataStoreEngine
    ( description
    , OrderedDouble(..)
    , mkOrderedDouble
    , SortedSet
    , emptySortedSet
    , sortedSetInsert
    , sortedSetRemove
    , sortedSetRank
    , sortedSetOrderedEntries
    , sortedSetRangeByIndex
    , sortedSetRangeByScore
    , EntryType(..)
    , entryTypeName
    , EntryValue(..)
    , Entry(..)
    , Database(..)
    , Store(..)
    , emptyStore
    , currentTimeMs
    , DataStoreBackend(..)
    , DataStoreEngine
    , newDataStoreEngine
    , executeFrame
    , executeOwned
    , executeWithDb
    , storeSnapshot
    , activeExpireAll
    ) where

import qualified Control.Concurrent.MVar as MVar
import Data.Char (toUpper)
import qualified Data.ByteString as BS
import qualified Data.ByteString.Char8 as BC
import qualified Data.Foldable as Foldable
import qualified Data.List as List
import Data.Maybe (isJust)
import qualified Data.Sequence as Seq
import Data.Sequence (Seq, ViewL(..), ViewR(..))
import qualified Data.Time.Clock.POSIX as POSIX
import Data.Word (Word8)
import qualified HashMap as HM
import qualified HashSet as HS
import qualified Heap
import qualified Hyperloglog as HLL
import InMemoryDataStoreProtocol
    ( CommandFrame(..)
    , EngineResponse(..)
    , commandFrameFromParts
    )
import Numeric (showFFloat)
import qualified RadixTree as RT
import qualified SkipList as Skip
import Text.Read (readMaybe)

description :: String
description = "Haskell in-memory data store engine without TCP transport"

newtype OrderedDouble = OrderedDouble
    { unwrapOrderedDouble :: Double
    }
    deriving (Show)

instance Eq OrderedDouble where
    OrderedDouble leftValue == OrderedDouble rightValue = leftValue == rightValue

instance Ord OrderedDouble where
    compare (OrderedDouble leftValue) (OrderedDouble rightValue) =
        compare leftValue rightValue

mkOrderedDouble :: Double -> Maybe OrderedDouble
mkOrderedDouble value
    | isNaN value = Nothing
    | otherwise = Just (OrderedDouble value)

data SortedEntry = SortedEntry
    { sortedEntryScore :: OrderedDouble
    , sortedEntryMember :: BS.ByteString
    }
    deriving (Eq, Ord, Show)

data SortedSet = SortedSet
    { sortedSetMembersMap :: HM.HashMap BS.ByteString OrderedDouble
    , sortedSetOrderingMap :: Skip.SkipList SortedEntry ()
    }
    deriving (Eq, Show)

emptySortedSet :: SortedSet
emptySortedSet =
    SortedSet
        { sortedSetMembersMap = HM.empty
        , sortedSetOrderingMap = Skip.empty
        }

sortedSetInsert :: Double -> BS.ByteString -> SortedSet -> Either String (SortedSet, Bool)
sortedSetInsert scoreValue memberValue valuesSet =
    case mkOrderedDouble scoreValue of
        Nothing -> Left "ERR value is not a valid float"
        Just orderedScore ->
            let existingScore = HM.get memberValue (sortedSetMembersMap valuesSet)
                strippedOrdering =
                    case existingScore of
                        Nothing -> sortedSetOrderingMap valuesSet
                        Just oldScore ->
                            Skip.delete (SortedEntry oldScore memberValue) (sortedSetOrderingMap valuesSet)
                updatedMembers =
                    HM.set memberValue orderedScore (sortedSetMembersMap valuesSet)
                updatedOrdering =
                    Skip.insert (SortedEntry orderedScore memberValue) () strippedOrdering
             in Right
                    ( SortedSet
                        { sortedSetMembersMap = updatedMembers
                        , sortedSetOrderingMap = updatedOrdering
                        }
                    , not (isJust existingScore)
                    )

sortedSetRemove :: BS.ByteString -> SortedSet -> (SortedSet, Bool)
sortedSetRemove memberValue valuesSet =
    case HM.get memberValue (sortedSetMembersMap valuesSet) of
        Nothing -> (valuesSet, False)
        Just oldScore ->
            ( SortedSet
                { sortedSetMembersMap =
                    HM.delete memberValue (sortedSetMembersMap valuesSet)
                , sortedSetOrderingMap =
                    Skip.delete
                        (SortedEntry oldScore memberValue)
                        (sortedSetOrderingMap valuesSet)
                }
            , True
            )

sortedSetRank :: BS.ByteString -> SortedSet -> Maybe Int
sortedSetRank memberValue valuesSet =
    List.findIndex
        ((== memberValue) . fst)
        (sortedSetOrderedEntries valuesSet)

sortedSetOrderedEntries :: SortedSet -> [(BS.ByteString, Double)]
sortedSetOrderedEntries valuesSet =
    [ (sortedEntryMember entry, unwrapOrderedDouble (sortedEntryScore entry))
    | (entry, ()) <- Skip.entries (sortedSetOrderingMap valuesSet)
    ]

sortedSetRangeByIndex :: Int -> Int -> SortedSet -> [(BS.ByteString, Double)]
sortedSetRangeByIndex startIndex endIndex valuesSet
    | null orderedEntries = []
    | normalizedStart < 0 = []
    | normalizedStart >= entryCount = []
    | normalizedEnd < 0 = []
    | clampedStart > clampedEnd = []
    | otherwise = take (clampedEnd - clampedStart + 1) (drop clampedStart orderedEntries)
  where
    orderedEntries = sortedSetOrderedEntries valuesSet
    entryCount = length orderedEntries
    normalizedStart = normalizeIndex entryCount startIndex
    normalizedEnd = normalizeIndex entryCount endIndex
    clampedStart = max 0 normalizedStart
    clampedEnd = min (entryCount - 1) normalizedEnd

sortedSetRangeByScore :: Double -> Double -> SortedSet -> [(BS.ByteString, Double)]
sortedSetRangeByScore minScore maxScore valuesSet =
    filter
        (\(_, scoreValue) -> scoreValue >= minScore && scoreValue <= maxScore)
        (sortedSetOrderedEntries valuesSet)

data EntryType
    = EntryString
    | EntryHash
    | EntryList
    | EntrySet
    | EntryZSet
    | EntryHll
    deriving (Eq, Ord, Show)

entryTypeName :: EntryType -> String
entryTypeName entryTypeValue =
    case entryTypeValue of
        EntryString -> "string"
        EntryHash -> "hash"
        EntryList -> "list"
        EntrySet -> "set"
        EntryZSet -> "zset"
        EntryHll -> "hll"

data EntryValue
    = EntryStringValue BS.ByteString
    | EntryHashValue (HM.HashMap BS.ByteString BS.ByteString)
    | EntryListValue (Seq BS.ByteString)
    | EntrySetValue (HS.HashSet BS.ByteString)
    | EntryZSetValue SortedSet
    | EntryHllValue HLL.HyperLogLog
    deriving (Eq, Show)

data Entry = Entry
    { entryTypeValue :: EntryType
    , entryValue :: EntryValue
    , entryExpiresAt :: Maybe Integer
    }
    deriving (Eq, Show)

data Database = Database
    { databaseEntries :: HM.HashMap BS.ByteString Entry
    , databaseTtlHeap :: Heap.MinHeap (Integer, BS.ByteString)
    , databaseKeyIndex :: RT.RadixTree ()
    }
    deriving (Eq, Show)

data Store = Store
    { storeDatabases :: [Database]
    , storeActiveDb :: Int
    }
    deriving (Eq, Show)

emptyDatabase :: Database
emptyDatabase =
    Database
        { databaseEntries = HM.empty
        , databaseTtlHeap = Heap.empty
        , databaseKeyIndex = RT.empty
        }

emptyStore :: Store
emptyStore =
    Store
        { storeDatabases = replicate 16 emptyDatabase
        , storeActiveDb = 0
        }

currentTimeMs :: IO Integer
currentTimeMs = floor . (* 1000) <$> POSIX.getPOSIXTime

data DataStoreEngine = DataStoreEngine
    { engineStoreVar :: MVar.MVar Store
    , engineAofPath :: Maybe FilePath
    }

class DataStoreBackend backend where
    executeBackendFrame :: backend -> CommandFrame -> IO EngineResponse
    executeBackendOwned :: backend -> [BS.ByteString] -> IO EngineResponse
    backendStoreSnapshot :: backend -> IO Store
    backendActiveExpireAll :: backend -> IO ()

instance DataStoreBackend DataStoreEngine where
    executeBackendFrame = executeFrame
    executeBackendOwned = executeOwned
    backendStoreSnapshot = storeSnapshot
    backendActiveExpireAll = activeExpireAll

newDataStoreEngine :: Maybe FilePath -> IO DataStoreEngine
newDataStoreEngine maybeAofPath = do
    storeVar <- MVar.newMVar emptyStore
    pure
        DataStoreEngine
            { engineStoreVar = storeVar
            , engineAofPath = maybeAofPath
            }

executeFrame :: DataStoreEngine -> CommandFrame -> IO EngineResponse
executeFrame engine commandFrameValue = snd <$> executeWithDb engine 0 commandFrameValue

executeOwned :: DataStoreEngine -> [BS.ByteString] -> IO EngineResponse
executeOwned engine parts =
    case commandFrameFromParts parts of
        Nothing ->
            pure (EngineError "ERR protocol error: expected array of bulk strings")
        Just commandFrameValue ->
            executeFrame engine commandFrameValue

executeWithDb :: DataStoreEngine -> Int -> CommandFrame -> IO (Int, EngineResponse)
executeWithDb engine requestedDb commandFrameValue = do
    now <- currentTimeMs
    MVar.modifyMVar
        (engineStoreVar engine)
        (\storeValue ->
            let commandName = map toUpper (commandFrameCommand commandFrameValue)
                storeWithDb = storeSelect requestedDb storeValue
                preparedStore =
                    if skipLazyExpire commandName
                        then storeWithDb
                        else storeExpireLazy now (listToMaybeByteString (commandFrameArgs commandFrameValue)) storeWithDb
                (updatedStore, response) =
                    dispatchCommand now preparedStore commandName (commandFrameArgs commandFrameValue)
             in pure (updatedStore, (storeActiveDb updatedStore, response)))

storeSnapshot :: DataStoreEngine -> IO Store
storeSnapshot engine = MVar.readMVar (engineStoreVar engine)

activeExpireAll :: DataStoreEngine -> IO ()
activeExpireAll engine = do
    now <- currentTimeMs
    MVar.modifyMVar_ (engineStoreVar engine) (pure . storeActiveExpireAll now)

entryFromValue :: EntryValue -> Maybe Integer -> Entry
entryFromValue value expiresAtValue =
    Entry
        { entryTypeValue = entryValueType value
        , entryValue = value
        , entryExpiresAt = expiresAtValue
        }

entryValueType :: EntryValue -> EntryType
entryValueType value =
    case value of
        EntryStringValue _ -> EntryString
        EntryHashValue _ -> EntryHash
        EntryListValue _ -> EntryList
        EntrySetValue _ -> EntrySet
        EntryZSetValue _ -> EntryZSet
        EntryHllValue _ -> EntryHll

databaseGet :: Integer -> BS.ByteString -> Database -> Maybe Entry
databaseGet now keyValue database =
    case HM.get keyValue (databaseEntries database) of
        Just entryValueFound
            | isExpired now entryValueFound -> Nothing
            | otherwise -> Just entryValueFound
        Nothing -> Nothing

databaseSet :: BS.ByteString -> Entry -> Database -> Database
databaseSet keyValue entryValueToStore database =
    database
        { databaseEntries = HM.set keyValue entryValueToStore (databaseEntries database)
        , databaseTtlHeap =
            case entryExpiresAt entryValueToStore of
                Nothing -> databaseTtlHeap database
                Just expiresAtValue ->
                    Heap.push (expiresAtValue, keyValue) (databaseTtlHeap database)
        , databaseKeyIndex = RT.insert keyValue () (databaseKeyIndex database)
        }

databaseDelete :: BS.ByteString -> Database -> Database
databaseDelete keyValue database =
    database
        { databaseEntries = HM.delete keyValue (databaseEntries database)
        , databaseKeyIndex = RT.delete keyValue (databaseKeyIndex database)
        }

databaseKeys :: Integer -> BS.ByteString -> Database -> [BS.ByteString]
databaseKeys now patternBytes database =
    List.sort
        [ keyValue
        | keyValue <- candidateKeys
        , isJust (databaseGet now keyValue database)
        , globMatch patternBytes keyValue
        ]
  where
    candidateKeys =
        case extractSimplePrefix patternBytes of
            Just prefixBytes -> RT.keysWithPrefix prefixBytes (databaseKeyIndex database)
            Nothing -> HM.keys (databaseEntries database)

databaseDbSize :: Integer -> Database -> Int
databaseDbSize now database =
    length [() | keyValue <- HM.keys (databaseEntries database), isJust (databaseGet now keyValue database)]

databaseExpireLazy :: Integer -> Maybe BS.ByteString -> Database -> Database
databaseExpireLazy now maybeKey database =
    case maybeKey of
        Nothing -> database
        Just keyValue ->
            case HM.get keyValue (databaseEntries database) of
                Just entryValueFound
                    | isExpired now entryValueFound -> databaseDelete keyValue database
                _ -> database

databaseActiveExpire :: Integer -> Database -> Database
databaseActiveExpire now database = loop database
  where
    loop currentDatabase =
        case Heap.minView (databaseTtlHeap currentDatabase) of
            Nothing -> currentDatabase
            Just ((expiresAtValue, keyValue), remainingHeap)
                | expiresAtValue > now -> currentDatabase
                | otherwise ->
                    let databaseWithoutTop = currentDatabase {databaseTtlHeap = remainingHeap}
                        shouldDelete =
                            case HM.get keyValue (databaseEntries databaseWithoutTop) of
                                Just entryValueFound ->
                                    entryExpiresAt entryValueFound == Just expiresAtValue
                                        && expiresAtValue <= now
                                Nothing -> False
                        nextDatabase =
                            if shouldDelete
                                then databaseDelete keyValue databaseWithoutTop
                                else databaseWithoutTop
                     in loop nextDatabase

databaseClear :: Database -> Database
databaseClear _ = emptyDatabase

storeSelect :: Int -> Store -> Store
storeSelect requestedDb storeValue =
    storeValue {storeActiveDb = clampDbIndex requestedDb}

storeGet :: Integer -> BS.ByteString -> Store -> Maybe Entry
storeGet now keyValue storeValue = databaseGet now keyValue (currentDatabase storeValue)

storeSet :: BS.ByteString -> Entry -> Store -> Store
storeSet keyValue entryValueToStore storeValue =
    updateCurrentDatabase (databaseSet keyValue entryValueToStore) storeValue

storeDelete :: BS.ByteString -> Store -> Store
storeDelete keyValue storeValue =
    updateCurrentDatabase (databaseDelete keyValue) storeValue

storeTypeOf :: Integer -> BS.ByteString -> Store -> Maybe EntryType
storeTypeOf now keyValue storeValue =
    entryTypeValue <$> storeGet now keyValue storeValue

storeKeys :: Integer -> BS.ByteString -> Store -> [BS.ByteString]
storeKeys now patternBytes storeValue =
    databaseKeys now patternBytes (currentDatabase storeValue)

storeDbSize :: Integer -> Store -> Int
storeDbSize now storeValue = databaseDbSize now (currentDatabase storeValue)

storeExpireLazy :: Integer -> Maybe BS.ByteString -> Store -> Store
storeExpireLazy now maybeKey storeValue =
    updateCurrentDatabase (databaseExpireLazy now maybeKey) storeValue

storeActiveExpireAll :: Integer -> Store -> Store
storeActiveExpireAll now storeValue =
    storeValue
        { storeDatabases =
            map (databaseActiveExpire now) (storeDatabases storeValue)
        }

storeFlushDb :: Store -> Store
storeFlushDb storeValue =
    updateCurrentDatabase databaseClear storeValue

storeFlushAll :: Store -> Store
storeFlushAll storeValue =
    storeValue {storeDatabases = replicate 16 emptyDatabase}

currentDatabase :: Store -> Database
currentDatabase storeValue = storeDatabases storeValue !! storeActiveDb storeValue

updateCurrentDatabase :: (Database -> Database) -> Store -> Store
updateCurrentDatabase updateFn storeValue =
    storeValue
        { storeDatabases =
            take activeIndex databasesValue
                ++ [updateFn (databasesValue !! activeIndex)]
                ++ drop (activeIndex + 1) databasesValue
        }
  where
    activeIndex = storeActiveDb storeValue
    databasesValue = storeDatabases storeValue

skipLazyExpire :: String -> Bool
skipLazyExpire commandName =
    commandName `elem` ["PING", "ECHO", "SELECT", "FLUSHDB", "FLUSHALL", "DBSIZE", "INFO"]

dispatchCommand :: Integer -> Store -> String -> [BS.ByteString] -> (Store, EngineResponse)
dispatchCommand now storeValue commandName args =
    case commandName of
        "PING" -> cmdPing storeValue args
        "ECHO" -> cmdEcho storeValue args
        "SET" -> cmdSet now storeValue args
        "GET" -> cmdGet now storeValue args
        "DEL" -> cmdDel now storeValue args
        "EXISTS" -> cmdExists now storeValue args
        "TYPE" -> cmdType now storeValue args
        "RENAME" -> cmdRename now storeValue args
        "INCR" -> cmdIncr now storeValue args
        "DECR" -> cmdDecr now storeValue args
        "INCRBY" -> cmdIncrBy now storeValue args
        "DECRBY" -> cmdDecrBy now storeValue args
        "APPEND" -> cmdAppend now storeValue args
        "HSET" -> cmdHSet now storeValue args
        "HGET" -> cmdHGet now storeValue args
        "HDEL" -> cmdHDel now storeValue args
        "HGETALL" -> cmdHGetAll now storeValue args
        "HLEN" -> cmdHLen now storeValue args
        "HEXISTS" -> cmdHExists now storeValue args
        "HKEYS" -> cmdHKeys now storeValue args
        "HVALS" -> cmdHVals now storeValue args
        "LPUSH" -> cmdLPush now storeValue args
        "RPUSH" -> cmdRPush now storeValue args
        "LPOP" -> cmdLPop now storeValue args
        "RPOP" -> cmdRPop now storeValue args
        "LLEN" -> cmdLLen now storeValue args
        "LRANGE" -> cmdLRange now storeValue args
        "LINDEX" -> cmdLIndex now storeValue args
        "SADD" -> cmdSAdd now storeValue args
        "SREM" -> cmdSRem now storeValue args
        "SISMEMBER" -> cmdSIsMember now storeValue args
        "SMEMBERS" -> cmdSMembers now storeValue args
        "SCARD" -> cmdSCard now storeValue args
        "SUNION" -> cmdSUnion now storeValue args
        "SINTER" -> cmdSInter now storeValue args
        "SDIFF" -> cmdSDiff now storeValue args
        "ZADD" -> cmdZAdd now storeValue args
        "ZRANGE" -> cmdZRange now storeValue args
        "ZRANGEBYSCORE" -> cmdZRangeByScore now storeValue args
        "ZRANK" -> cmdZRank now storeValue args
        "ZSCORE" -> cmdZScore now storeValue args
        "ZCARD" -> cmdZCard now storeValue args
        "ZREM" -> cmdZRem now storeValue args
        "PFADD" -> cmdPfAdd now storeValue args
        "PFCOUNT" -> cmdPfCount now storeValue args
        "PFMERGE" -> cmdPfMerge now storeValue args
        "EXPIRE" -> cmdExpire now storeValue args
        "EXPIREAT" -> cmdExpireAt now storeValue args
        "TTL" -> cmdTtl now storeValue args
        "PTTL" -> cmdPTtl now storeValue args
        "PERSIST" -> cmdPersist now storeValue args
        "SELECT" -> cmdSelect storeValue args
        "FLUSHDB" -> cmdFlushDb storeValue args
        "FLUSHALL" -> cmdFlushAll storeValue args
        "DBSIZE" -> cmdDbSize now storeValue args
        "INFO" -> cmdInfo now storeValue args
        "KEYS" -> cmdKeys now storeValue args
        _ -> (storeValue, EngineError ("ERR unknown command '" ++ commandName ++ "'"))

cmdPing :: Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdPing storeValue args =
    case args of
        [] -> (storeValue, EngineSimpleString "PONG")
        [messageValue] -> (storeValue, EngineBulkString (Just messageValue))
        _ -> (storeValue, wrongNumber "PING")

cmdEcho :: Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdEcho storeValue args =
    case args of
        [messageValue] -> (storeValue, EngineBulkString (Just messageValue))
        _ -> (storeValue, wrongNumber "ECHO")

cmdSet :: Integer -> Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdSet now storeValue args =
    case args of
        keyValue : valueBytes : optionBytes ->
            case parseSetOptions now optionBytes of
                Left errMessage -> (storeValue, EngineError errMessage)
                Right (expiresAtValue, shouldSetNx, shouldSetXx) ->
                    let keyExists = isJust (storeGet now keyValue storeValue)
                     in if shouldSetNx && keyExists
                            then (storeValue, EngineBulkString Nothing)
                            else if shouldSetXx && not keyExists
                                then (storeValue, EngineBulkString Nothing)
                                else
                                    ( storeSet keyValue (entryFromValue (EntryStringValue valueBytes) expiresAtValue) storeValue
                                    , okResponse
                                    )
        _ -> (storeValue, wrongNumber "SET")

cmdGet :: Integer -> Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdGet now storeValue args =
    case args of
        [keyValue] ->
            case storeGet now keyValue storeValue of
                Nothing -> (storeValue, EngineBulkString Nothing)
                Just entryValueFound ->
                    case entryValue entryValueFound of
                        EntryStringValue bytesValue ->
                            (storeValue, EngineBulkString (Just bytesValue))
                        _ -> (storeValue, wrongTypeResponse)
        _ -> (storeValue, wrongNumber "GET")

cmdDel :: Integer -> Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdDel now storeValue args
    | null args = (storeValue, wrongNumber "DEL")
    | otherwise =
        let (updatedStore, removedCount) =
                foldl
                    (\(currentStore, countValue) keyValue ->
                        if isJust (storeGet now keyValue currentStore)
                            then (storeDelete keyValue currentStore, countValue + 1)
                            else (currentStore, countValue))
                    (storeValue, 0 :: Integer)
                    args
         in (updatedStore, EngineInteger removedCount)

cmdExists :: Integer -> Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdExists now storeValue args
    | null args = (storeValue, wrongNumber "EXISTS")
    | otherwise =
        ( storeValue
        , EngineInteger
            (toInteger (length [() | keyValue <- args, isJust (storeGet now keyValue storeValue)]))
        )

cmdType :: Integer -> Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdType now storeValue args =
    case args of
        [keyValue] ->
            ( storeValue
            , EngineSimpleString
                (maybe "none" entryTypeName (storeTypeOf now keyValue storeValue))
            )
        _ -> (storeValue, wrongNumber "TYPE")

cmdRename :: Integer -> Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdRename now storeValue args =
    case args of
        [sourceKey, destinationKey] ->
            case storeGet now sourceKey storeValue of
                Nothing -> (storeValue, EngineError "ERR no such key")
                Just entryValueFound ->
                    let updatedStore =
                            storeSet
                                destinationKey
                                entryValueFound
                                (storeDelete sourceKey storeValue)
                     in (updatedStore, okResponse)
        _ -> (storeValue, wrongNumber "RENAME")

cmdIncr :: Integer -> Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdIncr now storeValue args =
    case args of
        [keyValue] -> adjustInteger now storeValue keyValue 1
        _ -> (storeValue, wrongNumber "INCR")

cmdDecr :: Integer -> Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdDecr now storeValue args =
    case args of
        [keyValue] -> adjustInteger now storeValue keyValue (-1)
        _ -> (storeValue, wrongNumber "DECR")

cmdIncrBy :: Integer -> Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdIncrBy now storeValue args =
    case args of
        [keyValue, deltaBytes] ->
            case parseIntegerArg deltaBytes of
                Left errMessage -> (storeValue, EngineError errMessage)
                Right deltaValue -> adjustInteger now storeValue keyValue deltaValue
        _ -> (storeValue, wrongNumber "INCRBY")

cmdDecrBy :: Integer -> Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdDecrBy now storeValue args =
    case args of
        [keyValue, deltaBytes] ->
            case parseIntegerArg deltaBytes of
                Left errMessage -> (storeValue, EngineError errMessage)
                Right deltaValue -> adjustInteger now storeValue keyValue (negate deltaValue)
        _ -> (storeValue, wrongNumber "DECRBY")

cmdAppend :: Integer -> Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdAppend now storeValue args =
    case args of
        [keyValue, suffixValue] ->
            case storeGet now keyValue storeValue of
                Nothing ->
                    ( storeSet keyValue (entryFromValue (EntryStringValue suffixValue) Nothing) storeValue
                    , EngineInteger (toInteger (BS.length suffixValue))
                    )
                Just entryValueFound ->
                    case entryValue entryValueFound of
                        EntryStringValue existingValue ->
                            let combinedValue = existingValue <> suffixValue
                                updatedStore =
                                    storeSet
                                        keyValue
                                        (entryFromValue (EntryStringValue combinedValue) (entryExpiresAt entryValueFound))
                                        storeValue
                             in (updatedStore, EngineInteger (toInteger (BS.length combinedValue)))
                        _ -> (storeValue, wrongTypeResponse)
        _ -> (storeValue, wrongNumber "APPEND")

cmdHSet :: Integer -> Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdHSet now storeValue args
    | length args < 3 || even (length args) =
        (storeValue, wrongNumber "HSET")
    | otherwise =
        let keyValue = head args
            valuePairs = pairs (tail args)
         in case loadHash now storeValue keyValue of
                Left response -> (storeValue, response)
                Right (hashValueMap, expiresAtValue) ->
                    let (updatedHash, addedCount) =
                            foldl
                                (\(currentHash, countValue) (fieldValue, fieldBytes) ->
                                    let isNewField = not (HM.has fieldValue currentHash)
                                     in (HM.set fieldValue fieldBytes currentHash, countValue + if isNewField then 1 else 0))
                                (hashValueMap, 0 :: Integer)
                                valuePairs
                     in ( storeSet keyValue (entryFromValue (EntryHashValue updatedHash) expiresAtValue) storeValue
                        , EngineInteger addedCount
                        )

cmdHGet :: Integer -> Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdHGet now storeValue args =
    case args of
        [keyValue, fieldValue] ->
            case loadHash now storeValue keyValue of
                Left response -> (storeValue, response)
                Right (hashValueMap, _) ->
                    (storeValue, EngineBulkString (HM.get fieldValue hashValueMap))
        _ -> (storeValue, wrongNumber "HGET")

cmdHDel :: Integer -> Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdHDel now storeValue args
    | length args < 2 = (storeValue, wrongNumber "HDEL")
    | otherwise =
        let keyValue = head args
            fields = tail args
         in case loadHash now storeValue keyValue of
                Left response -> (storeValue, response)
                Right (hashValueMap, expiresAtValue) ->
                    let (updatedHash, removedCount) =
                            foldl
                                (\(currentHash, countValue) fieldValue ->
                                    if HM.has fieldValue currentHash
                                        then (HM.delete fieldValue currentHash, countValue + 1)
                                        else (currentHash, countValue))
                                (hashValueMap, 0 :: Integer)
                                fields
                        updatedStore =
                            if HM.isEmpty updatedHash
                                then storeDelete keyValue storeValue
                                else storeSet keyValue (entryFromValue (EntryHashValue updatedHash) expiresAtValue) storeValue
                     in (updatedStore, EngineInteger removedCount)

cmdHGetAll :: Integer -> Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdHGetAll now storeValue args =
    case args of
        [keyValue] ->
            case loadHash now storeValue keyValue of
                Left response -> (storeValue, response)
                Right (hashValueMap, _) ->
                    (storeValue, EngineArray (Just (concatMap kvResponse (HM.entries hashValueMap))))
        _ -> (storeValue, wrongNumber "HGETALL")

cmdHLen :: Integer -> Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdHLen now storeValue args =
    case args of
        [keyValue] ->
            case loadHash now storeValue keyValue of
                Left response -> (storeValue, response)
                Right (hashValueMap, _) ->
                    (storeValue, EngineInteger (toInteger (HM.size hashValueMap)))
        _ -> (storeValue, wrongNumber "HLEN")

cmdHExists :: Integer -> Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdHExists now storeValue args =
    case args of
        [keyValue, fieldValue] ->
            case loadHash now storeValue keyValue of
                Left response -> (storeValue, response)
                Right (hashValueMap, _) ->
                    (storeValue, EngineInteger (if HM.has fieldValue hashValueMap then 1 else 0))
        _ -> (storeValue, wrongNumber "HEXISTS")

cmdHKeys :: Integer -> Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdHKeys now storeValue args =
    case args of
        [keyValue] ->
            case loadHash now storeValue keyValue of
                Left response -> (storeValue, response)
                Right (hashValueMap, _) ->
                    (storeValue, EngineArray (Just [EngineBulkString (Just fieldValue) | fieldValue <- HM.keys hashValueMap]))
        _ -> (storeValue, wrongNumber "HKEYS")

cmdHVals :: Integer -> Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdHVals now storeValue args =
    case args of
        [keyValue] ->
            case loadHash now storeValue keyValue of
                Left response -> (storeValue, response)
                Right (hashValueMap, _) ->
                    ( storeValue
                    , EngineArray
                        (Just [EngineBulkString (Just valueBytes) | (_, valueBytes) <- HM.entries hashValueMap])
                    )
        _ -> (storeValue, wrongNumber "HVALS")

cmdLPush :: Integer -> Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdLPush now storeValue args
    | length args < 2 = (storeValue, wrongNumber "LPUSH")
    | otherwise =
        let keyValue = head args
            valuesToPush = tail args
         in case loadList now storeValue keyValue of
                Left response -> (storeValue, response)
                Right (listValue, expiresAtValue) ->
                    let updatedList = foldl (flip (Seq.<|)) listValue valuesToPush
                     in ( storeSet keyValue (entryFromValue (EntryListValue updatedList) expiresAtValue) storeValue
                        , EngineInteger (toInteger (Foldable.length updatedList))
                        )

cmdRPush :: Integer -> Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdRPush now storeValue args
    | length args < 2 = (storeValue, wrongNumber "RPUSH")
    | otherwise =
        let keyValue = head args
            valuesToPush = tail args
         in case loadList now storeValue keyValue of
                Left response -> (storeValue, response)
                Right (listValue, expiresAtValue) ->
                    let updatedList = Foldable.foldl' (Seq.|>) listValue valuesToPush
                     in ( storeSet keyValue (entryFromValue (EntryListValue updatedList) expiresAtValue) storeValue
                        , EngineInteger (toInteger (Foldable.length updatedList))
                        )

cmdLPop :: Integer -> Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdLPop now storeValue args =
    case args of
        [keyValue] ->
            case loadList now storeValue keyValue of
                Left response -> (storeValue, response)
                Right (listValue, expiresAtValue) ->
                    case Seq.viewl listValue of
                        EmptyL -> (storeDelete keyValue storeValue, EngineBulkString Nothing)
                        valueBytes Seq.:< restList ->
                            let updatedStore =
                                    if Seq.null restList
                                        then storeDelete keyValue storeValue
                                        else storeSet keyValue (entryFromValue (EntryListValue restList) expiresAtValue) storeValue
                             in (updatedStore, EngineBulkString (Just valueBytes))
        _ -> (storeValue, wrongNumber "LPOP")

cmdRPop :: Integer -> Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdRPop now storeValue args =
    case args of
        [keyValue] ->
            case loadList now storeValue keyValue of
                Left response -> (storeValue, response)
                Right (listValue, expiresAtValue) ->
                    case Seq.viewr listValue of
                        EmptyR -> (storeDelete keyValue storeValue, EngineBulkString Nothing)
                        restList Seq.:> valueBytes ->
                            let updatedStore =
                                    if Seq.null restList
                                        then storeDelete keyValue storeValue
                                        else storeSet keyValue (entryFromValue (EntryListValue restList) expiresAtValue) storeValue
                             in (updatedStore, EngineBulkString (Just valueBytes))
        _ -> (storeValue, wrongNumber "RPOP")

cmdLLen :: Integer -> Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdLLen now storeValue args =
    case args of
        [keyValue] ->
            case loadList now storeValue keyValue of
                Left response -> (storeValue, response)
                Right (listValue, _) ->
                    (storeValue, EngineInteger (toInteger (Foldable.length listValue)))
        _ -> (storeValue, wrongNumber "LLEN")

cmdLRange :: Integer -> Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdLRange now storeValue args =
    case args of
        [keyValue, startBytes, endBytes] ->
            case (parseIntegerArg startBytes, parseIntegerArg endBytes) of
                (Right startValue, Right endValue) ->
                    case loadList now storeValue keyValue of
                        Left response -> (storeValue, response)
                        Right (listValue, _) ->
                            let listedValues = Foldable.toList listValue
                                sliceValues = sliceRange listedValues (fromInteger startValue) (fromInteger endValue)
                             in (storeValue, EngineArray (Just (map (EngineBulkString . Just) sliceValues)))
                (Left errMessage, _) -> (storeValue, EngineError errMessage)
                (_, Left errMessage) -> (storeValue, EngineError errMessage)
        _ -> (storeValue, wrongNumber "LRANGE")

cmdLIndex :: Integer -> Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdLIndex now storeValue args =
    case args of
        [keyValue, indexBytes] ->
            case parseIntegerArg indexBytes of
                Left errMessage -> (storeValue, EngineError errMessage)
                Right indexValue ->
                    case loadList now storeValue keyValue of
                        Left response -> (storeValue, response)
                        Right (listValue, _) ->
                            let listedValues = Foldable.toList listValue
                                normalizedValue = normalizeIndex (length listedValues) (fromInteger indexValue)
                             in if normalizedValue < 0 || normalizedValue >= length listedValues
                                    then (storeValue, EngineBulkString Nothing)
                                    else (storeValue, EngineBulkString (Just (listedValues !! normalizedValue)))
        _ -> (storeValue, wrongNumber "LINDEX")

cmdSAdd :: Integer -> Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdSAdd now storeValue args
    | length args < 2 = (storeValue, wrongNumber "SADD")
    | otherwise =
        let keyValue = head args
            membersToAdd = tail args
         in case loadSet now storeValue keyValue of
                Left response -> (storeValue, response)
                Right (setValue, expiresAtValue) ->
                    let (updatedSet, addedCount) =
                            foldl
                                (\(currentSet, countValue) memberValue ->
                                    if HS.contains memberValue currentSet
                                        then (currentSet, countValue)
                                        else (HS.add memberValue currentSet, countValue + 1))
                                (setValue, 0 :: Integer)
                                membersToAdd
                     in ( storeSet keyValue (entryFromValue (EntrySetValue updatedSet) expiresAtValue) storeValue
                        , EngineInteger addedCount
                        )

cmdSRem :: Integer -> Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdSRem now storeValue args
    | length args < 2 = (storeValue, wrongNumber "SREM")
    | otherwise =
        let keyValue = head args
            membersToRemove = tail args
         in case loadSet now storeValue keyValue of
                Left response -> (storeValue, response)
                Right (setValue, expiresAtValue) ->
                    let (updatedSet, removedCount) =
                            foldl
                                (\(currentSet, countValue) memberValue ->
                                    if HS.contains memberValue currentSet
                                        then (HS.remove memberValue currentSet, countValue + 1)
                                        else (currentSet, countValue))
                                (setValue, 0 :: Integer)
                                membersToRemove
                        updatedStore =
                            if HS.isEmpty updatedSet
                                then storeDelete keyValue storeValue
                                else storeSet keyValue (entryFromValue (EntrySetValue updatedSet) expiresAtValue) storeValue
                     in (updatedStore, EngineInteger removedCount)

cmdSIsMember :: Integer -> Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdSIsMember now storeValue args =
    case args of
        [keyValue, memberValue] ->
            case loadSet now storeValue keyValue of
                Left response -> (storeValue, response)
                Right (setValue, _) ->
                    (storeValue, EngineInteger (if HS.contains memberValue setValue then 1 else 0))
        _ -> (storeValue, wrongNumber "SISMEMBER")

cmdSMembers :: Integer -> Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdSMembers now storeValue args =
    case args of
        [keyValue] ->
            case loadSet now storeValue keyValue of
                Left response -> (storeValue, response)
                Right (setValue, _) ->
                    (storeValue, EngineArray (Just [EngineBulkString (Just memberValue) | memberValue <- HS.toList setValue]))
        _ -> (storeValue, wrongNumber "SMEMBERS")

cmdSCard :: Integer -> Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdSCard now storeValue args =
    case args of
        [keyValue] ->
            case loadSet now storeValue keyValue of
                Left response -> (storeValue, response)
                Right (setValue, _) ->
                    (storeValue, EngineInteger (toInteger (HS.size setValue)))
        _ -> (storeValue, wrongNumber "SCARD")

cmdSUnion :: Integer -> Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdSUnion now storeValue args
    | null args = (storeValue, wrongNumber "SUNION")
    | otherwise =
        case traverse (loadSetForRead now storeValue) args of
            Left response -> (storeValue, response)
            Right setsValue ->
                let combinedSet = foldl HS.union HS.empty setsValue
                 in (storeValue, EngineArray (Just [EngineBulkString (Just memberValue) | memberValue <- HS.toList combinedSet]))

cmdSInter :: Integer -> Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdSInter now storeValue args
    | null args = (storeValue, wrongNumber "SINTER")
    | otherwise =
        case traverse (loadSetForRead now storeValue) args of
            Left response -> (storeValue, response)
            Right [] -> (storeValue, EngineArray (Just []))
            Right (firstSet : restSets) ->
                let combinedSet = foldl HS.intersection firstSet restSets
                 in (storeValue, EngineArray (Just [EngineBulkString (Just memberValue) | memberValue <- HS.toList combinedSet]))

cmdSDiff :: Integer -> Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdSDiff now storeValue args
    | null args = (storeValue, wrongNumber "SDIFF")
    | otherwise =
        case traverse (loadSetForRead now storeValue) args of
            Left response -> (storeValue, response)
            Right [] -> (storeValue, EngineArray (Just []))
            Right (firstSet : restSets) ->
                let combinedSet = foldl HS.difference firstSet restSets
                 in (storeValue, EngineArray (Just [EngineBulkString (Just memberValue) | memberValue <- HS.toList combinedSet]))

cmdZAdd :: Integer -> Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdZAdd now storeValue args
    | length args < 3 || even (length args) = (storeValue, wrongNumber "ZADD")
    | otherwise =
        let keyValue = head args
            scorePairs = pairs (tail args)
         in case loadZSet now storeValue keyValue of
                Left response -> (storeValue, response)
                Right (zsetValue, expiresAtValue) ->
                    case foldl insertPair (Right (zsetValue, 0 :: Integer)) scorePairs of
                        Left errMessage -> (storeValue, EngineError errMessage)
                        Right (updatedZSet, addedCount) ->
                            ( storeSet keyValue (entryFromValue (EntryZSetValue updatedZSet) expiresAtValue) storeValue
                            , EngineInteger addedCount
                            )
  where
    insertPair accumulator (scoreBytes, memberValue) =
        case accumulator of
            Left errMessage -> Left errMessage
            Right (currentZSet, countValue) ->
                case parseDoubleArg scoreBytes of
                    Left errMessage -> Left errMessage
                    Right scoreValue ->
                        case sortedSetInsert scoreValue memberValue currentZSet of
                            Left errMessage -> Left errMessage
                            Right (nextZSet, isNewMember) ->
                                Right (nextZSet, countValue + if isNewMember then 1 else 0)

cmdZRange :: Integer -> Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdZRange now storeValue args =
    case args of
        [keyValue, startBytes, endBytes] ->
            case (parseIntegerArg startBytes, parseIntegerArg endBytes) of
                (Right startValue, Right endValue) ->
                    case loadZSet now storeValue keyValue of
                        Left response -> (storeValue, response)
                        Right (zsetValue, _) ->
                            let members =
                                    [ EngineBulkString (Just memberValue)
                                    | (memberValue, _) <- sortedSetRangeByIndex (fromInteger startValue) (fromInteger endValue) zsetValue
                                    ]
                             in (storeValue, EngineArray (Just members))
                (Left errMessage, _) -> (storeValue, EngineError errMessage)
                (_, Left errMessage) -> (storeValue, EngineError errMessage)
        _ -> (storeValue, wrongNumber "ZRANGE")

cmdZRangeByScore :: Integer -> Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdZRangeByScore now storeValue args =
    case args of
        [keyValue, minBytes, maxBytes] ->
            case (parseDoubleArg minBytes, parseDoubleArg maxBytes) of
                (Right minValue, Right maxValue) ->
                    case loadZSet now storeValue keyValue of
                        Left response -> (storeValue, response)
                        Right (zsetValue, _) ->
                            let members =
                                    [ EngineBulkString (Just memberValue)
                                    | (memberValue, _) <- sortedSetRangeByScore minValue maxValue zsetValue
                                    ]
                             in (storeValue, EngineArray (Just members))
                (Left errMessage, _) -> (storeValue, EngineError errMessage)
                (_, Left errMessage) -> (storeValue, EngineError errMessage)
        _ -> (storeValue, wrongNumber "ZRANGEBYSCORE")

cmdZRank :: Integer -> Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdZRank now storeValue args =
    case args of
        [keyValue, memberValue] ->
            case loadZSet now storeValue keyValue of
                Left response -> (storeValue, response)
                Right (zsetValue, _) ->
                    case sortedSetRank memberValue zsetValue of
                        Nothing -> (storeValue, EngineBulkString Nothing)
                        Just rankValue -> (storeValue, EngineInteger (toInteger rankValue))
        _ -> (storeValue, wrongNumber "ZRANK")

cmdZScore :: Integer -> Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdZScore now storeValue args =
    case args of
        [keyValue, memberValue] ->
            case loadZSet now storeValue keyValue of
                Left response -> (storeValue, response)
                Right (zsetValue, _) ->
                    let maybeScore = lookup memberValue (sortedSetOrderedEntries zsetValue)
                     in (storeValue, EngineBulkString (renderScore <$> maybeScore))
        _ -> (storeValue, wrongNumber "ZSCORE")

cmdZCard :: Integer -> Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdZCard now storeValue args =
    case args of
        [keyValue] ->
            case loadZSet now storeValue keyValue of
                Left response -> (storeValue, response)
                Right (zsetValue, _) ->
                    (storeValue, EngineInteger (toInteger (length (sortedSetOrderedEntries zsetValue))))
        _ -> (storeValue, wrongNumber "ZCARD")

cmdZRem :: Integer -> Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdZRem now storeValue args
    | length args < 2 = (storeValue, wrongNumber "ZREM")
    | otherwise =
        let keyValue = head args
            membersToRemove = tail args
         in case loadZSet now storeValue keyValue of
                Left response -> (storeValue, response)
                Right (zsetValue, expiresAtValue) ->
                    let (updatedZSet, removedCount) =
                            foldl
                                (\(currentZSet, countValue) memberValue ->
                                    let (nextZSet, removed) = sortedSetRemove memberValue currentZSet
                                     in (nextZSet, countValue + if removed then 1 else 0))
                                (zsetValue, 0 :: Integer)
                                membersToRemove
                        updatedStore =
                            if null (sortedSetOrderedEntries updatedZSet)
                                then storeDelete keyValue storeValue
                                else storeSet keyValue (entryFromValue (EntryZSetValue updatedZSet) expiresAtValue) storeValue
                     in (updatedStore, EngineInteger removedCount)

cmdPfAdd :: Integer -> Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdPfAdd now storeValue args
    | length args < 2 = (storeValue, wrongNumber "PFADD")
    | otherwise =
        let keyValue = head args
            valuesToAdd = tail args
         in case loadHll now storeValue keyValue of
                Left response -> (storeValue, response)
                Right (hllValue, expiresAtValue) ->
                    let (updatedHll, changed) = HLL.addMany valuesToAdd hllValue
                     in ( storeSet keyValue (entryFromValue (EntryHllValue updatedHll) expiresAtValue) storeValue
                        , EngineInteger (if changed then 1 else 0)
                        )

cmdPfCount :: Integer -> Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdPfCount now storeValue args
    | null args = (storeValue, wrongNumber "PFCOUNT")
    | otherwise =
        case traverse (loadHllForRead now storeValue) args of
            Left response -> (storeValue, response)
            Right hllValues ->
                let mergedHll = HLL.mergeMany hllValues
                 in (storeValue, EngineInteger (HLL.count mergedHll))

cmdPfMerge :: Integer -> Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdPfMerge now storeValue args
    | length args < 2 = (storeValue, wrongNumber "PFMERGE")
    | otherwise =
        let destinationKey = head args
            sourceKeys = tail args
         in case traverse (loadHllForRead now storeValue) sourceKeys of
                Left response -> (storeValue, response)
                Right hllValues ->
                    case loadHll now storeValue destinationKey of
                        Left response -> (storeValue, response)
                        Right (_, expiresAtValue) ->
                            let mergedHll = HLL.mergeMany hllValues
                             in ( storeSet destinationKey (entryFromValue (EntryHllValue mergedHll) expiresAtValue) storeValue
                                , okResponse
                                )

cmdExpire :: Integer -> Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdExpire now storeValue args =
    case args of
        [keyValue, secondsBytes] ->
            case parseIntegerArg secondsBytes of
                Left errMessage -> (storeValue, EngineError errMessage)
                Right secondsValue ->
                    setExpiry now storeValue keyValue (Just (now + secondsValue * 1000))
        _ -> (storeValue, wrongNumber "EXPIRE")

cmdExpireAt :: Integer -> Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdExpireAt now storeValue args =
    case args of
        [keyValue, timestampBytes] ->
            case parseIntegerArg timestampBytes of
                Left errMessage -> (storeValue, EngineError errMessage)
                Right timestampValue ->
                    setExpiry now storeValue keyValue (Just (timestampValue * 1000))
        _ -> (storeValue, wrongNumber "EXPIREAT")

cmdTtl :: Integer -> Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdTtl now storeValue args =
    case args of
        [keyValue] ->
            case ttlForKey now keyValue storeValue of
                Nothing -> (storeValue, EngineInteger (-2))
                Just Nothing -> (storeValue, EngineInteger (-1))
                Just (Just expiresAtValue) ->
                    (storeValue, EngineInteger (max 0 ((expiresAtValue - now) `div` 1000)))
        _ -> (storeValue, wrongNumber "TTL")

cmdPTtl :: Integer -> Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdPTtl now storeValue args =
    case args of
        [keyValue] ->
            case ttlForKey now keyValue storeValue of
                Nothing -> (storeValue, EngineInteger (-2))
                Just Nothing -> (storeValue, EngineInteger (-1))
                Just (Just expiresAtValue) ->
                    (storeValue, EngineInteger (max 0 (expiresAtValue - now)))
        _ -> (storeValue, wrongNumber "PTTL")

cmdPersist :: Integer -> Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdPersist now storeValue args =
    case args of
        [keyValue] ->
            case storeGet now keyValue storeValue of
                Nothing -> (storeValue, EngineInteger 0)
                Just entryValueFound ->
                    case entryExpiresAt entryValueFound of
                        Nothing -> (storeValue, EngineInteger 0)
                        Just _ ->
                            ( storeSet
                                keyValue
                                (entryFromValue (entryValue entryValueFound) Nothing)
                                storeValue
                            , EngineInteger 1
                            )
        _ -> (storeValue, wrongNumber "PERSIST")

cmdSelect :: Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdSelect storeValue args =
    case args of
        [dbBytes] ->
            case parseIntegerArg dbBytes of
                Left errMessage -> (storeValue, EngineError errMessage)
                Right dbValue
                    | dbValue < 0 -> (storeValue, EngineError "ERR DB index is out of range")
                    | otherwise -> (storeSelect (fromInteger dbValue) storeValue, okResponse)
        _ -> (storeValue, wrongNumber "SELECT")

cmdFlushDb :: Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdFlushDb storeValue args =
    case args of
        [] -> (storeFlushDb storeValue, okResponse)
        _ -> (storeValue, wrongNumber "FLUSHDB")

cmdFlushAll :: Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdFlushAll storeValue args =
    case args of
        [] -> (storeFlushAll storeValue, okResponse)
        _ -> (storeValue, wrongNumber "FLUSHALL")

cmdDbSize :: Integer -> Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdDbSize now storeValue args =
    case args of
        [] -> (storeValue, EngineInteger (toInteger (storeDbSize now storeValue)))
        _ -> (storeValue, wrongNumber "DBSIZE")

cmdInfo :: Integer -> Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdInfo now storeValue args =
    case args of
        [] ->
            let currentDbValue = currentDatabase storeValue
                expiresCount =
                    length
                        [ ()
                        | (_, entryValueFound) <- HM.entries (databaseEntries currentDbValue)
                        , isJust (entryExpiresAt entryValueFound)
                        ]
                infoText =
                    unlines
                        [ "# Server"
                        , "mini_redis_haskell:1"
                        , "# Keyspace"
                        , "db" ++ show (storeActiveDb storeValue)
                            ++ ":keys="
                            ++ show (storeDbSize now storeValue)
                            ++ ",expires="
                            ++ show expiresCount
                        ]
             in (storeValue, EngineBulkString (Just (BC.pack infoText)))
        _ -> (storeValue, wrongNumber "INFO")

cmdKeys :: Integer -> Store -> [BS.ByteString] -> (Store, EngineResponse)
cmdKeys now storeValue args =
    case args of
        [patternBytes] ->
            ( storeValue
            , EngineArray
                (Just [EngineBulkString (Just keyValue) | keyValue <- storeKeys now patternBytes storeValue])
            )
        _ -> (storeValue, wrongNumber "KEYS")

loadHash :: Integer -> Store -> BS.ByteString -> Either EngineResponse (HM.HashMap BS.ByteString BS.ByteString, Maybe Integer)
loadHash now storeValue keyValue =
    case storeGet now keyValue storeValue of
        Nothing -> Right (HM.empty, Nothing)
        Just entryValueFound ->
            case entryValue entryValueFound of
                EntryHashValue hashValueMap -> Right (hashValueMap, entryExpiresAt entryValueFound)
                _ -> Left wrongTypeResponse

loadList :: Integer -> Store -> BS.ByteString -> Either EngineResponse (Seq BS.ByteString, Maybe Integer)
loadList now storeValue keyValue =
    case storeGet now keyValue storeValue of
        Nothing -> Right (Seq.empty, Nothing)
        Just entryValueFound ->
            case entryValue entryValueFound of
                EntryListValue listValue -> Right (listValue, entryExpiresAt entryValueFound)
                _ -> Left wrongTypeResponse

loadSet :: Integer -> Store -> BS.ByteString -> Either EngineResponse (HS.HashSet BS.ByteString, Maybe Integer)
loadSet now storeValue keyValue =
    case storeGet now keyValue storeValue of
        Nothing -> Right (HS.empty, Nothing)
        Just entryValueFound ->
            case entryValue entryValueFound of
                EntrySetValue setValue -> Right (setValue, entryExpiresAt entryValueFound)
                _ -> Left wrongTypeResponse

loadSetForRead :: Integer -> Store -> BS.ByteString -> Either EngineResponse (HS.HashSet BS.ByteString)
loadSetForRead now storeValue keyValue =
    case storeGet now keyValue storeValue of
        Nothing -> Right HS.empty
        Just entryValueFound ->
            case entryValue entryValueFound of
                EntrySetValue setValue -> Right setValue
                _ -> Left wrongTypeResponse

loadZSet :: Integer -> Store -> BS.ByteString -> Either EngineResponse (SortedSet, Maybe Integer)
loadZSet now storeValue keyValue =
    case storeGet now keyValue storeValue of
        Nothing -> Right (emptySortedSet, Nothing)
        Just entryValueFound ->
            case entryValue entryValueFound of
                EntryZSetValue zsetValue -> Right (zsetValue, entryExpiresAt entryValueFound)
                _ -> Left wrongTypeResponse

loadHll :: Integer -> Store -> BS.ByteString -> Either EngineResponse (HLL.HyperLogLog, Maybe Integer)
loadHll now storeValue keyValue =
    case storeGet now keyValue storeValue of
        Nothing -> Right (HLL.new, Nothing)
        Just entryValueFound ->
            case entryValue entryValueFound of
                EntryHllValue hllValue -> Right (hllValue, entryExpiresAt entryValueFound)
                _ -> Left wrongTypeResponse

loadHllForRead :: Integer -> Store -> BS.ByteString -> Either EngineResponse HLL.HyperLogLog
loadHllForRead now storeValue keyValue =
    case storeGet now keyValue storeValue of
        Nothing -> Right HLL.new
        Just entryValueFound ->
            case entryValue entryValueFound of
                EntryHllValue hllValue -> Right hllValue
                _ -> Left wrongTypeResponse

adjustInteger :: Integer -> Store -> BS.ByteString -> Integer -> (Store, EngineResponse)
adjustInteger now storeValue keyValue deltaValue =
    case storeGet now keyValue storeValue of
        Nothing ->
            let nextValue = deltaValue
             in ( storeSet
                    keyValue
                    (entryFromValue (EntryStringValue (BC.pack (show nextValue))) Nothing)
                    storeValue
                , EngineInteger nextValue
                )
        Just entryValueFound ->
            case entryValue entryValueFound of
                EntryStringValue currentBytes ->
                    case parseIntegerArg currentBytes of
                        Left errMessage -> (storeValue, EngineError errMessage)
                        Right currentValue ->
                            let nextValue = currentValue + deltaValue
                                updatedStore =
                                    storeSet
                                        keyValue
                                        (entryFromValue
                                            (EntryStringValue (BC.pack (show nextValue)))
                                            (entryExpiresAt entryValueFound))
                                        storeValue
                             in (updatedStore, EngineInteger nextValue)
                _ -> (storeValue, wrongTypeResponse)

parseSetOptions :: Integer -> [BS.ByteString] -> Either String (Maybe Integer, Bool, Bool)
parseSetOptions now = go Nothing False False
  where
    go expiresAtValue useNx useXx optionBytes =
        case optionBytes of
            [] ->
                if useNx && useXx
                    then Left "ERR syntax error"
                    else Right (expiresAtValue, useNx, useXx)
            optionValue : restOptions ->
                case map toUpper (BC.unpack optionValue) of
                    "EX" ->
                        case restOptions of
                            nextValue : remaining ->
                                parseIntegerArg nextValue >>= \secondsValue ->
                                    go (Just (now + secondsValue * 1000)) useNx useXx remaining
                            _ -> Left "ERR syntax error"
                    "PX" ->
                        case restOptions of
                            nextValue : remaining ->
                                parseIntegerArg nextValue >>= \millisValue ->
                                    go (Just (now + millisValue)) useNx useXx remaining
                            _ -> Left "ERR syntax error"
                    "NX" -> go expiresAtValue True useXx restOptions
                    "XX" -> go expiresAtValue useNx True restOptions
                    _ -> Left "ERR syntax error"

setExpiry :: Integer -> Store -> BS.ByteString -> Maybe Integer -> (Store, EngineResponse)
setExpiry now storeValue keyValue expiresAtValue =
    case storeGet now keyValue storeValue of
        Nothing -> (storeValue, EngineInteger 0)
        Just entryValueFound ->
            ( storeSet
                keyValue
                (entryFromValue (entryValue entryValueFound) expiresAtValue)
                storeValue
            , EngineInteger 1
            )

ttlForKey :: Integer -> BS.ByteString -> Store -> Maybe (Maybe Integer)
ttlForKey now keyValue storeValue =
    case storeGet now keyValue storeValue of
        Nothing -> Nothing
        Just entryValueFound -> Just (entryExpiresAt entryValueFound)

wrongNumber :: String -> EngineResponse
wrongNumber commandName =
    EngineError ("ERR wrong number of arguments for '" ++ commandName ++ "'")

wrongTypeResponse :: EngineResponse
wrongTypeResponse =
    EngineError "WRONGTYPE Operation against a key holding the wrong kind of value"

okResponse :: EngineResponse
okResponse = EngineSimpleString "OK"

parseIntegerArg :: BS.ByteString -> Either String Integer
parseIntegerArg bytesValue =
    case BC.readInteger bytesValue of
        Just (numberValue, rest) | BS.null rest -> Right numberValue
        _ -> Left "ERR value is not an integer or out of range"

parseDoubleArg :: BS.ByteString -> Either String Double
parseDoubleArg bytesValue =
    case readMaybe (BC.unpack bytesValue) of
        Just numberValue | not (isNaN numberValue) -> Right numberValue
        _ -> Left "ERR value is not a valid float"

renderScore :: Double -> BS.ByteString
renderScore scoreValue
    | isWholeNumber scoreValue = BC.pack (show (round scoreValue :: Integer))
    | otherwise = BC.pack (trimTrailingZeros (showFFloat Nothing scoreValue ""))

trimTrailingZeros :: String -> String
trimTrailingZeros textValue =
    case span (/= '.') textValue of
        (_, "") -> textValue
        _ ->
            let trimmed = reverse (dropWhile (== '0') (reverse textValue))
             in if not (null trimmed) && last trimmed == '.'
                    then init trimmed
                    else trimmed

isWholeNumber :: Double -> Bool
isWholeNumber scoreValue =
    let roundedValue = fromInteger (round scoreValue)
     in abs (scoreValue - roundedValue) < 1.0e-12

normalizeIndex :: Int -> Int -> Int
normalizeIndex totalLength indexValue
    | indexValue < 0 = totalLength + indexValue
    | otherwise = indexValue

sliceRange :: [value] -> Int -> Int -> [value]
sliceRange valuesList startIndex endIndex
    | null valuesList = []
    | normalizedStart < 0 = []
    | normalizedStart >= totalLength = []
    | normalizedEnd < 0 = []
    | clampedStart > clampedEnd = []
    | otherwise = take (clampedEnd - clampedStart + 1) (drop clampedStart valuesList)
  where
    totalLength = length valuesList
    normalizedStart = normalizeIndex totalLength startIndex
    normalizedEnd = normalizeIndex totalLength endIndex
    clampedStart = max 0 normalizedStart
    clampedEnd = min (totalLength - 1) normalizedEnd

pairs :: [value] -> [(value, value)]
pairs valuesList =
    case valuesList of
        firstValue : secondValue : restValues -> (firstValue, secondValue) : pairs restValues
        _ -> []

kvResponse :: (BS.ByteString, BS.ByteString) -> [EngineResponse]
kvResponse (keyValue, valueBytes) =
    [EngineBulkString (Just keyValue), EngineBulkString (Just valueBytes)]

listToMaybeByteString :: [BS.ByteString] -> Maybe BS.ByteString
listToMaybeByteString valuesList =
    case valuesList of
        [] -> Nothing
        value : _ -> Just value

clampDbIndex :: Int -> Int
clampDbIndex requestedDb = max 0 (min 15 requestedDb)

isExpired :: Integer -> Entry -> Bool
isExpired now entryValueFound =
    case entryExpiresAt entryValueFound of
        Just expiresAtValue -> now >= expiresAtValue
        Nothing -> False

extractSimplePrefix :: BS.ByteString -> Maybe BS.ByteString
extractSimplePrefix patternBytes =
    case BS.elemIndex 42 patternBytes of
        Nothing ->
            if BS.any isWildcard patternBytes
                then Nothing
                else Just patternBytes
        Just wildcardIndex ->
            let prefixBytes = BS.take wildcardIndex patternBytes
                restBytes = BS.drop (wildcardIndex + 1) patternBytes
             in if BS.any isWildcard prefixBytes || not (BS.null restBytes)
                    then Nothing
                    else Just prefixBytes
  where
    isWildcard byteValue = byteValue == 42 || byteValue == 63 || byteValue == 91

globMatch :: BS.ByteString -> BS.ByteString -> Bool
globMatch patternBytes textBytes = go (BS.unpack patternBytes) (BS.unpack textBytes)
  where
    go [] [] = True
    go [] _ = False
    go (42 : remainingPattern) textValues =
        go remainingPattern textValues
            || case textValues of
                [] -> False
                _ : remainingText -> go (42 : remainingPattern) remainingText
    go (63 : remainingPattern) (_ : remainingText) = go remainingPattern remainingText
    go (63 : _) [] = False
    go (91 : remainingPattern) textValues =
        case break (== 93) remainingPattern of
            (classValues, 93 : restPattern) ->
                case textValues of
                    textByte : remainingText ->
                        classContains classValues textByte && go restPattern remainingText
                    [] -> False
            _ ->
                case textValues of
                    textByte : remainingText ->
                        textByte == 91 && go remainingPattern remainingText
                    [] -> False
    go (patternByte : remainingPattern) (textByte : remainingText) =
        patternByte == textByte && go remainingPattern remainingText
    go _ _ = False

classContains :: [Word8] -> Word8 -> Bool
classContains classValues textByte = go classValues
  where
    go valuesList =
        case valuesList of
            startValue : 45 : endValue : restValues ->
                (startValue <= textByte && textByte <= endValue) || go restValues
            value : restValues -> value == textByte || go restValues
            [] -> False
