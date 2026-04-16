module RespProtocolSpec (spec) where

import qualified Data.ByteString.Char8 as BC
import RespProtocol
import Test.Hspec

spec :: Spec
spec = describe "RespProtocol" $ do
    it "encodes scalar values and arrays" $ do
        encodeSimpleString "OK" `shouldBe` Right (BC.pack "+OK\r\n")
        encode (RespInteger 7) `shouldBe` Right (BC.pack ":7\r\n")
        encode (RespBulkString (Just (BC.pack "abc")))
            `shouldBe` Right (BC.pack "$3\r\nabc\r\n")
        encode (RespArray (Just [RespSimpleString "OK", RespInteger 1]))
            `shouldBe` Right (BC.pack "*2\r\n+OK\r\n:1\r\n")

    it "rejects newlines in simple strings" $ do
        encodeSimpleString "bad\nnews"
            `shouldSatisfy` (\result -> case result of Left _ -> True; Right _ -> False)

    it "decodes scalar values, arrays, and inline commands" $ do
        decode (BC.pack "+OK\r\n")
            `shouldBe` Right (Just (RespSimpleString "OK", 5))
        decode (BC.pack "$3\r\nfoo\r\n")
            `shouldBe` Right (Just (RespBulkString (Just (BC.pack "foo")), 9))
        decode (BC.pack "*2\r\n+OK\r\n:1\r\n")
            `shouldBe`
                Right
                    (Just
                        ( RespArray (Just [RespSimpleString "OK", RespInteger 1])
                        , 13
                        ))
        decode (BC.pack "PING hello\r\n")
            `shouldBe`
                Right
                    (Just
                        ( RespArray
                            (Just
                                [ RespBulkString (Just (BC.pack "PING"))
                                , RespBulkString (Just (BC.pack "hello"))
                                ])
                        , 12
                        ))

    it "decodes multiple messages from one buffer" $ do
        decodeAll (BC.pack "+OK\r\n:1\r\n")
            `shouldBe` Right ([RespSimpleString "OK", RespInteger 1], 9)
