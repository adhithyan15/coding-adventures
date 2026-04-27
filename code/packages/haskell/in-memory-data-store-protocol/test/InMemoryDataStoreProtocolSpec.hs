module InMemoryDataStoreProtocolSpec (spec) where

import qualified Data.ByteString.Char8 as BC
import InMemoryDataStoreProtocol
import RespProtocol
import Test.Hspec

spec :: Spec
spec = describe "InMemoryDataStoreProtocol" $ do
    it "builds command frames from parts and RESP arrays" $ do
        commandFrameFromParts (map BC.pack ["set", "alpha", "1"])
            `shouldBe`
                Just
                    (CommandFrame "SET" (map BC.pack ["alpha", "1"]))
        commandFrameFromResp
            (RespArray
                (Just
                    [ RespBulkString (Just (BC.pack "get"))
                    , RespBulkString (Just (BC.pack "alpha"))
                    ]))
            `shouldBe`
                Just
                    (CommandFrame "GET" [BC.pack "alpha"])

    it "converts engine responses back into RESP values" $ do
        engineResponseToResp
            (EngineArray
                (Just
                    [ EngineSimpleString "OK"
                    , EngineBulkString (Just (BC.pack "value"))
                    ]))
            `shouldBe`
                RespArray
                    (Just
                        [ RespSimpleString "OK"
                        , RespBulkString (Just (BC.pack "value"))
                        ])
