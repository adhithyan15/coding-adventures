module InMemoryDataStoreSpec (spec) where

import qualified Data.ByteString.Char8 as BC
import InMemoryDataStore
import Test.Hspec

spec :: Spec
spec = describe "InMemoryDataStore" $ do
    it "processes RESP command bytes without a TCP server" $ do
        manager <- newDataStoreManager Nothing
        let setBytes = BC.pack "*3\r\n$3\r\nSET\r\n$5\r\nalpha\r\n$1\r\n1\r\n"
            getBytes = BC.pack "*2\r\n$3\r\nGET\r\n$5\r\nalpha\r\n"
        (_, setResponses) <- executeRespBytes manager 0 setBytes
        (_, getResponses) <- executeRespBytes manager 0 getBytes
        setResponses `shouldBe` [EngineSimpleString "OK"]
        getResponses `shouldBe` [EngineBulkString (Just (BC.pack "1"))]
        encodeResponses getResponses `shouldBe` Right (BC.pack "$1\r\n1\r\n")

    it "starts and stops background workers safely" $ do
        manager <- newDataStoreManager Nothing
        startBackgroundWorkers manager
        stopBackgroundWorkers manager
        description `shouldSatisfy` (not . null)
