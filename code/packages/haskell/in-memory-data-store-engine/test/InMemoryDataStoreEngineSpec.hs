module InMemoryDataStoreEngineSpec (spec) where

import qualified Data.ByteString.Char8 as BC
import InMemoryDataStoreEngine
import InMemoryDataStoreProtocol
import Test.Hspec

spec :: Spec
spec = describe "InMemoryDataStoreEngine" $ do
    it "executes string, hash, set, and sorted-set commands" $ do
        engine <- newDataStoreEngine Nothing
        executeOwned engine (map BC.pack ["PING"])
            `shouldReturn` EngineSimpleString "PONG"
        executeOwned engine (map BC.pack ["SET", "alpha", "1"])
            `shouldReturn` EngineSimpleString "OK"
        executeOwned engine (map BC.pack ["GET", "alpha"])
            `shouldReturn` EngineBulkString (Just (BC.pack "1"))
        executeOwned engine (map BC.pack ["HSET", "hash", "b", "2", "a", "1"])
            `shouldReturn` EngineInteger 2
        executeOwned engine (map BC.pack ["HGETALL", "hash"])
            `shouldReturn`
                EngineArray
                    (Just
                        [ EngineBulkString (Just (BC.pack "a"))
                        , EngineBulkString (Just (BC.pack "1"))
                        , EngineBulkString (Just (BC.pack "b"))
                        , EngineBulkString (Just (BC.pack "2"))
                        ])
        executeOwned engine (map BC.pack ["SADD", "set", "c", "a", "b"])
            `shouldReturn` EngineInteger 3
        executeOwned engine (map BC.pack ["SMEMBERS", "set"])
            `shouldReturn`
                EngineArray
                    (Just
                        [ EngineBulkString (Just (BC.pack "a"))
                        , EngineBulkString (Just (BC.pack "b"))
                        , EngineBulkString (Just (BC.pack "c"))
                        ])
        executeOwned engine (map BC.pack ["ZADD", "scores", "2", "b", "1", "a"])
            `shouldReturn` EngineInteger 2
        executeOwned engine (map BC.pack ["ZRANGE", "scores", "0", "-1"])
            `shouldReturn`
                EngineArray
                    (Just
                        [ EngineBulkString (Just (BC.pack "a"))
                        , EngineBulkString (Just (BC.pack "b"))
                        ])
        executeOwned engine (map BC.pack ["ZRANK", "scores", "b"])
            `shouldReturn` EngineInteger 1
        executeOwned engine (map BC.pack ["ZSCORE", "scores", "a"])
            `shouldReturn` EngineBulkString (Just (BC.pack "1"))

    it "tracks ttl, databases, and hyperloglog state without tcp" $ do
        engine <- newDataStoreEngine Nothing
        executeWithDb engine 0 (CommandFrame "SET" (map BC.pack ["db:key", "value"]))
            `shouldReturn` (0, EngineSimpleString "OK")
        executeOwned engine (map BC.pack ["PFADD", "visitors", "a", "b", "c"])
            `shouldReturn` EngineInteger 1
        executeOwned engine (map BC.pack ["PFCOUNT", "visitors"])
            `shouldReturn` EngineInteger 3
        executeOwned engine (map BC.pack ["DBSIZE"])
            `shouldReturn` EngineInteger 2
        executeOwned engine (map BC.pack ["EXPIRE", "db:key", "0"])
            `shouldReturn` EngineInteger 1
        activeExpireAll engine
        executeOwned engine (map BC.pack ["GET", "db:key"])
            `shouldReturn` EngineBulkString Nothing

    it "covers keyspace helpers and wrongtype errors" $ do
        engine <- newDataStoreEngine Nothing
        mapM_ (\keyName -> executeOwned engine (map BC.pack ["SET", keyName, "hello"])) ["foo_a", "foo_b", "key_x"]
        executeOwned engine (map BC.pack ["KEYS", "foo*"])
            `shouldReturn`
                EngineArray
                    (Just
                        [ EngineBulkString (Just (BC.pack "foo_a"))
                        , EngineBulkString (Just (BC.pack "foo_b"))
                        ])
        executeOwned engine (map BC.pack ["LPUSH", "wrongtype", "foo"])
            `shouldReturn` EngineInteger 1
        executeOwned engine (map BC.pack ["HSET", "wrongtype", "bar", "baz"])
            `shouldReturn`
                EngineError "WRONGTYPE Operation against a key holding the wrong kind of value"
