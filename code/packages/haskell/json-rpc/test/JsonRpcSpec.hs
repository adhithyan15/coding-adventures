module JsonRpcSpec (spec) where

import qualified Data.ByteString.Char8 as BSC
import Data.IORef
import Test.Hspec

import JsonRpc

spec :: Spec
spec = do
    describe "parseMessage" $ do
        it "parses a request with params" $ do
            let payload = BSC.pack "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\",\"params\":{\"value\":3}}"
            parseMessage payload
                `shouldBe` Right
                    (RequestMessage
                        (Request
                            (JsonNumber 1)
                            "ping"
                            (Just (JsonObject [("value", JsonNumber 3)]))
                        )
                    )

        it "parses a notification" $ do
            let payload = BSC.pack "{\"jsonrpc\":\"2.0\",\"method\":\"initialized\"}"
            parseMessage payload
                `shouldBe` Right (NotificationMessage (Notification "initialized" Nothing))

        it "returns a parse error for invalid JSON" $
            parseMessage (BSC.pack "not json")
                `shouldBe` Left (parseErrorResponse Nothing)

    describe "messageToValue" $
        it "serialises error responses without a result field" $ do
            let value =
                    messageToValue
                        (ResponseMessage (Response (JsonNumber 1) Nothing (Just (methodNotFoundResponse "hover"))))
            value
                `shouldBe` JsonObject
                    [ ("jsonrpc", JsonString "2.0")
                    , ("id", JsonNumber 1)
                    , ("error", JsonObject [("code", JsonNumber (fromIntegral methodNotFoundCode)), ("message", JsonString "Method not found"), ("data", JsonString "hover")])
                    ]

    describe "framing" $ do
        it "writes a Content-Length frame" $ do
            let framed = renderMessageFrame (NotificationMessage (Notification "initialized" Nothing))
            BSC.isPrefixOf (BSC.pack "Content-Length: ") framed `shouldBe` True

        it "round-trips multiple framed messages" $ do
            let first = NotificationMessage (Notification "one" Nothing)
                second = RequestMessage (Request (JsonNumber 2) "two" Nothing)
                payload = renderMessageFrame first <> renderMessageFrame second
            parseFramedMessages payload `shouldBe` Right [first, second]

    describe "dispatchMessage" $ do
        it "dispatches registered request handlers" $ do
            let server =
                    onRequest "ping" (\_ _ -> pure (Right (JsonObject [("ok", JsonBool True)]))) emptyServer
            dispatchMessage server (RequestMessage (Request (JsonNumber 1) "ping" Nothing))
                `shouldReturn` Just (ResponseMessage (Response (JsonNumber 1) (Just (JsonObject [("ok", JsonBool True)])) Nothing))

        it "returns method-not-found for unknown requests" $ do
            let server = emptyServer
            dispatchMessage server (RequestMessage (Request (JsonNumber 1) "missing" Nothing))
                `shouldReturn` Just (ResponseMessage (Response (JsonNumber 1) Nothing (Just (methodNotFoundResponse "missing"))))

        it "runs notification handlers without returning a response" $ do
            seen <- newIORef ([] :: [JsonValue])
            let server =
                    onNotification
                        "opened"
                        (\params -> writeIORef seen [maybe JsonNull id params])
                        emptyServer
            dispatchMessage server (NotificationMessage (Notification "opened" (Just (JsonString "file.txt"))))
                `shouldReturn` Nothing
            readIORef seen `shouldReturn` [JsonString "file.txt"]

    describe "serveByteString" $
        it "produces framed responses for requests and parse errors" $ do
            let validRequest = renderMessageFrame (RequestMessage (Request (JsonNumber 9) "ping" Nothing))
                invalidRequest = framePayload (BSC.pack "{bad json")
                server =
                    onRequest "ping" (\_ _ -> pure (Right (JsonString "pong"))) emptyServer
            result <- serveByteString server (validRequest <> invalidRequest)
            case result of
                Left err -> expectationFailure err
                Right output ->
                    parseFramedMessages output
                        `shouldBe` Right
                            [ ResponseMessage (Response (JsonNumber 9) (Just (JsonString "pong")) Nothing)
                            , ResponseMessage (Response JsonNull Nothing (Just (parseErrorResponse Nothing)))
                            ]
