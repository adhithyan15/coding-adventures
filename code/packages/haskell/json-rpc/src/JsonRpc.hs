module JsonRpc
    ( JsonValue(..)
    , parseErrorCode
    , invalidRequestCode
    , methodNotFoundCode
    , invalidParamsCode
    , internalErrorCode
    , ResponseError(..)
    , parseErrorResponse
    , invalidRequestResponse
    , methodNotFoundResponse
    , invalidParamsResponse
    , internalErrorResponse
    , Id
    , Request(..)
    , Response(..)
    , Notification(..)
    , Message(..)
    , parseJson
    , renderJson
    , parseMessage
    , messageToValue
    , encodeMessagePayload
    , framePayload
    , renderMessageFrame
    , extractFrame
    , parseFramedMessages
    , RequestHandler
    , NotificationHandler
    , Server
    , emptyServer
    , onRequest
    , onNotification
    , dispatchMessage
    , serveByteString
    , serveHandles
    ) where

import Control.Applicative ((<|>))
import qualified Data.ByteString as BS
import qualified Data.ByteString.Char8 as BSC
import qualified Data.Map.Strict as Map
import Data.Map.Strict (Map)
import Data.Char (chr, digitToInt, isDigit, ord)
import Data.List (intercalate)
import Data.Maybe (mapMaybe)
import Numeric (showHex)
import System.IO (Handle, hFlush)
import Text.ParserCombinators.ReadP
    ( ReadP
    , between
    , char
    , choice
    , eof
    , many
    , munch1
    , option
    , pfail
    , readP_to_S
    , satisfy
    , sepBy
    , skipSpaces
    , string
    )

data JsonValue
    = JsonNull
    | JsonBool Bool
    | JsonNumber Double
    | JsonString String
    | JsonArray [JsonValue]
    | JsonObject [(String, JsonValue)]
    deriving (Eq, Show)

parseErrorCode, invalidRequestCode, methodNotFoundCode, invalidParamsCode, internalErrorCode :: Int
parseErrorCode = -32700
invalidRequestCode = -32600
methodNotFoundCode = -32601
invalidParamsCode = -32602
internalErrorCode = -32603

data ResponseError = ResponseError
    { responseErrorCode :: Int
    , responseErrorMessage :: String
    , responseErrorData :: Maybe JsonValue
    }
    deriving (Eq, Show)

type Id = JsonValue

data Request = Request
    { requestId :: Id
    , requestMethod :: String
    , requestParams :: Maybe JsonValue
    }
    deriving (Eq, Show)

data Response = Response
    { responseId :: Id
    , responseResult :: Maybe JsonValue
    , responseError :: Maybe ResponseError
    }
    deriving (Eq, Show)

data Notification = Notification
    { notificationMethod :: String
    , notificationParams :: Maybe JsonValue
    }
    deriving (Eq, Show)

data Message
    = RequestMessage Request
    | ResponseMessage Response
    | NotificationMessage Notification
    deriving (Eq, Show)

parseErrorResponse :: Maybe JsonValue -> ResponseError
parseErrorResponse =
    ResponseError parseErrorCode "Parse error"

invalidRequestResponse :: Maybe JsonValue -> ResponseError
invalidRequestResponse =
    ResponseError invalidRequestCode "Invalid Request"

methodNotFoundResponse :: String -> ResponseError
methodNotFoundResponse methodName =
    ResponseError methodNotFoundCode "Method not found" (Just (JsonString methodName))

invalidParamsResponse :: Maybe JsonValue -> ResponseError
invalidParamsResponse =
    ResponseError invalidParamsCode "Invalid params"

internalErrorResponse :: Maybe JsonValue -> ResponseError
internalErrorResponse =
    ResponseError internalErrorCode "Internal error"

parseJson :: String -> Either String JsonValue
parseJson input =
    case [value | (value, rest) <- readP_to_S (skipSpaces *> jsonValueParser <* skipSpaces <* eof) input, null rest] of
        [] -> Left "invalid json"
        values -> Right (last values)

renderJson :: JsonValue -> String
renderJson value =
    case value of
        JsonNull -> "null"
        JsonBool True -> "true"
        JsonBool False -> "false"
        JsonNumber numberValue
            | isWholeNumber numberValue -> show (round numberValue :: Integer)
            | otherwise -> show numberValue
        JsonString textValue -> "\"" ++ concatMap escapeJsonChar textValue ++ "\""
        JsonArray values -> "[" ++ intercalate "," (map renderJson values) ++ "]"
        JsonObject fields ->
            "{"
                ++ intercalate
                    ","
                    [ renderJson (JsonString fieldName) ++ ":" ++ renderJson fieldValue
                    | (fieldName, fieldValue) <- fields
                    ]
                ++ "}"

parseMessage :: BS.ByteString -> Either ResponseError Message
parseMessage payload =
    case parseJson (BSC.unpack payload) of
        Left _ -> Left (parseErrorResponse Nothing)
        Right value -> parseMessageValue value

messageToValue :: Message -> JsonValue
messageToValue message =
    case message of
        RequestMessage req ->
            JsonObject $
                [ ("jsonrpc", JsonString "2.0")
                , ("id", requestId req)
                , ("method", JsonString (requestMethod req))
                ]
                    ++ maybe [] (\params -> [("params", params)]) (requestParams req)
        NotificationMessage notif ->
            JsonObject $
                [ ("jsonrpc", JsonString "2.0")
                , ("method", JsonString (notificationMethod notif))
                ]
                    ++ maybe [] (\params -> [("params", params)]) (notificationParams notif)
        ResponseMessage resp ->
            JsonObject $
                [ ("jsonrpc", JsonString "2.0")
                , ("id", responseId resp)
                ]
                    ++ maybe [] (\resultValue -> [("result", resultValue)]) (responseResult resp)
                    ++ maybe [] (\err -> [("error", responseErrorToValue err)]) (responseError resp)

encodeMessagePayload :: Message -> BS.ByteString
encodeMessagePayload =
    BSC.pack . renderJson . messageToValue

framePayload :: BS.ByteString -> BS.ByteString
framePayload payload =
    BSC.pack ("Content-Length: " ++ show (BS.length payload) ++ "\r\n\r\n") <> payload

renderMessageFrame :: Message -> BS.ByteString
renderMessageFrame =
    framePayload . encodeMessagePayload

extractFrame :: BS.ByteString -> Either String (Maybe (BS.ByteString, BS.ByteString))
extractFrame input
    | BS.null input = Right Nothing
    | otherwise =
        case findHeaderSeparator input of
            Nothing -> Left "missing header separator"
            Just headerLength -> do
                let headerBytes = BS.take headerLength input
                    remaining = BS.drop (headerLength + 4) input
                contentLength <- parseContentLength headerBytes
                if BS.length remaining < contentLength
                    then Left "incomplete frame"
                    else Right (Just (BS.take contentLength remaining, BS.drop contentLength remaining))

parseFramedMessages :: BS.ByteString -> Either String [Message]
parseFramedMessages =
    go []
  where
    go acc buffer =
        case extractFrame buffer of
            Left err -> Left err
            Right Nothing -> Right (reverse acc)
            Right (Just (payload, rest)) ->
                case parseMessage payload of
                    Left err -> Left ("invalid payload: " ++ responseErrorMessage err)
                    Right message -> go (message : acc) rest

type RequestHandler = Id -> Maybe JsonValue -> IO (Either ResponseError JsonValue)
type NotificationHandler = Maybe JsonValue -> IO ()

data Server = Server
    { serverRequestHandlers :: Map String RequestHandler
    , serverNotificationHandlers :: Map String NotificationHandler
    }

emptyServer :: Server
emptyServer =
    Server
        { serverRequestHandlers = Map.empty
        , serverNotificationHandlers = Map.empty
        }

onRequest :: String -> RequestHandler -> Server -> Server
onRequest methodName handler server =
    server{serverRequestHandlers = Map.insert methodName handler (serverRequestHandlers server)}

onNotification :: String -> NotificationHandler -> Server -> Server
onNotification methodName handler server =
    server{serverNotificationHandlers = Map.insert methodName handler (serverNotificationHandlers server)}

dispatchMessage :: Server -> Message -> IO (Maybe Message)
dispatchMessage server message =
    case message of
        ResponseMessage _ ->
            pure Nothing
        NotificationMessage notif ->
            case Map.lookup (notificationMethod notif) (serverNotificationHandlers server) of
                Nothing -> pure Nothing
                Just handler -> handler (notificationParams notif) >> pure Nothing
        RequestMessage req ->
            case Map.lookup (requestMethod req) (serverRequestHandlers server) of
                Nothing ->
                    pure
                        (Just
                            (ResponseMessage
                                (Response
                                    (requestId req)
                                    Nothing
                                    (Just (methodNotFoundResponse (requestMethod req)))
                                )
                            )
                        )
                Just handler -> do
                    result <- handler (requestId req) (requestParams req)
                    pure . Just $
                        ResponseMessage $
                            case result of
                                Left err -> Response (requestId req) Nothing (Just err)
                                Right value -> Response (requestId req) (Just value) Nothing

serveByteString :: Server -> BS.ByteString -> IO (Either String BS.ByteString)
serveByteString server input = do
    responses <- collectResponses [] input
    pure (fmap (BS.concat . map renderMessageFrame) responses)
  where
    collectResponses acc buffer =
        case extractFrame buffer of
            Left err -> pure (Left err)
            Right Nothing -> pure (Right (reverse acc))
            Right (Just (payload, rest)) ->
                case parseMessage payload of
                    Left err ->
                        collectResponses
                            (ResponseMessage (Response JsonNull Nothing (Just err)) : acc)
                            rest
                    Right message -> do
                        maybeResponse <- dispatchMessage server message
                        collectResponses (maybe acc (: acc) maybeResponse) rest

serveHandles :: Server -> Handle -> Handle -> IO (Either String ())
serveHandles server inputHandle outputHandle = do
    payload <- BS.hGetContents inputHandle
    result <- serveByteString server payload
    case result of
        Left err -> pure (Left err)
        Right output -> do
            BS.hPut outputHandle output
            hFlush outputHandle
            pure (Right ())

parseMessageValue :: JsonValue -> Either ResponseError Message
parseMessageValue (JsonObject fields) = do
    ensureJsonRpcVersion fields
    let hasId = hasField "id" fields
        hasMethod = hasField "method" fields
        hasResult = hasField "result" fields
        hasError = hasField "error" fields
    case (hasId, hasMethod, hasResult, hasError) of
        (True, True, False, False) -> parseRequest fields
        (False, True, False, False) -> parseNotification fields
        (_, False, _, _) | hasResult || hasError -> parseResponse fields
        _ -> Left (invalidRequestResponse Nothing)
parseMessageValue _ =
    Left (invalidRequestResponse Nothing)

parseRequest :: [(String, JsonValue)] -> Either ResponseError Message
parseRequest fields = do
    ident <- requiredField "id" fields
    if ident == JsonNull
        then Left (invalidRequestResponse (Just (JsonString "request id must not be null")))
        else do
            methodName <- requiredString "method" fields
            pure (RequestMessage (Request ident methodName (lookupField "params" fields)))

parseNotification :: [(String, JsonValue)] -> Either ResponseError Message
parseNotification fields = do
    methodName <- requiredString "method" fields
    pure (NotificationMessage (Notification methodName (lookupField "params" fields)))

parseResponse :: [(String, JsonValue)] -> Either ResponseError Message
parseResponse fields = do
    ident <- requiredField "id" fields
    let resultValue = lookupField "result" fields
        errorValue = lookupField "error" fields
    case (resultValue, errorValue) of
        (Just _, Just _) -> Left (invalidRequestResponse (Just (JsonString "response cannot contain both result and error")))
        (Nothing, Nothing) -> Left (invalidRequestResponse (Just (JsonString "response must contain result or error")))
        (Just value, Nothing) ->
            pure (ResponseMessage (Response ident (Just value) Nothing))
        (Nothing, Just value) ->
            case responseErrorFromValue value of
                Left err -> Left err
                Right responseErr -> pure (ResponseMessage (Response ident Nothing (Just responseErr)))

ensureJsonRpcVersion :: [(String, JsonValue)] -> Either ResponseError ()
ensureJsonRpcVersion fields =
    case lookupField "jsonrpc" fields of
        Just (JsonString "2.0") -> Right ()
        _ -> Left (invalidRequestResponse (Just (JsonString "jsonrpc must be \"2.0\"")))

responseErrorToValue :: ResponseError -> JsonValue
responseErrorToValue err =
    JsonObject $
        [ ("code", JsonNumber (fromIntegral (responseErrorCode err)))
        , ("message", JsonString (responseErrorMessage err))
        ]
            ++ maybe [] (\value -> [("data", value)]) (responseErrorData err)

responseErrorFromValue :: JsonValue -> Either ResponseError ResponseError
responseErrorFromValue (JsonObject fields) = do
    codeValue <- requiredField "code" fields
    codeNumber <-
        case codeValue of
            JsonNumber value -> Right (round value)
            _ -> Left (invalidRequestResponse (Just (JsonString "error code must be numeric")))
    messageValue <- requiredString "message" fields
    pure
        (ResponseError
            { responseErrorCode = codeNumber
            , responseErrorMessage = messageValue
            , responseErrorData = lookupField "data" fields
            }
        )
responseErrorFromValue _ =
    Left (invalidRequestResponse (Just (JsonString "error must be an object")))

requiredField :: String -> [(String, JsonValue)] -> Either ResponseError JsonValue
requiredField name fields =
    maybe (Left (invalidRequestResponse (Just (JsonString ("missing field " ++ name))))) Right (lookupField name fields)

requiredString :: String -> [(String, JsonValue)] -> Either ResponseError String
requiredString name fields =
    case lookupField name fields of
        Just (JsonString value) -> Right value
        _ -> Left (invalidRequestResponse (Just (JsonString ("invalid string field " ++ name))))

lookupField :: String -> [(String, JsonValue)] -> Maybe JsonValue
lookupField name fields =
    lookup name fields

hasField :: String -> [(String, JsonValue)] -> Bool
hasField name fields =
    case lookupField name fields of
        Nothing -> False
        Just _ -> True

parseContentLength :: BS.ByteString -> Either String Int
parseContentLength headerBytes =
    case lookup "Content-Length" headerPairs of
        Nothing -> Left "missing Content-Length header"
        Just rawValue ->
            case reads rawValue of
                [(count, "")] | count >= 0 -> Right count
                _ -> Left "invalid Content-Length header"
  where
    headerPairs =
        mapMaybe parseHeaderLine (BSC.lines headerBytes)

parseHeaderLine :: BS.ByteString -> Maybe (String, String)
parseHeaderLine line =
    case break (== ':') (BSC.unpack (stripTrailingCarriageReturn line)) of
        (_, []) -> Nothing
        (name, _ : value) -> Just (name, dropWhile (== ' ') value)

findHeaderSeparator :: BS.ByteString -> Maybe Int
findHeaderSeparator =
    go 0
  where
    marker = BSC.pack "\r\n\r\n"
    markerLength = BS.length marker

    go index bytes
        | BS.length bytes < markerLength = Nothing
        | marker `BS.isPrefixOf` bytes = Just index
        | otherwise = go (index + 1) (BS.tail bytes)

stripTrailingCarriageReturn :: BS.ByteString -> BS.ByteString
stripTrailingCarriageReturn =
    BS.dropWhileEnd (== 13)

jsonValueParser :: ReadP JsonValue
jsonValueParser =
    skipSpaces *> choice
        [ JsonNull <$ string "null"
        , JsonBool True <$ string "true"
        , JsonBool False <$ string "false"
        , JsonString <$> jsonStringLiteral
        , JsonArray <$> between (char '[' *> skipSpaces) (skipSpaces *> char ']') (jsonValueParser `sepBy` (skipSpaces *> char ',' *> skipSpaces))
        , JsonObject <$> between (char '{' *> skipSpaces) (skipSpaces *> char '}') (jsonPairParser `sepBy` (skipSpaces *> char ',' *> skipSpaces))
        , JsonNumber <$> jsonNumberParser
        ]

jsonPairParser :: ReadP (String, JsonValue)
jsonPairParser = do
    key <- jsonStringLiteral
    skipSpaces
    _ <- char ':'
    skipSpaces
    value <- jsonValueParser
    pure (key, value)

jsonStringLiteral :: ReadP String
jsonStringLiteral =
    between (char '"') (char '"') (many jsonCharacterParser)

jsonCharacterParser :: ReadP Char
jsonCharacterParser =
    escaped <|> plain
  where
    plain = satisfyNot ['"', '\\']
    escaped = do
        _ <- char '\\'
        choice
            [ '"' <$ char '"'
            , '\\' <$ char '\\'
            , '/' <$ char '/'
            , '\b' <$ char 'b'
            , '\f' <$ char 'f'
            , '\n' <$ char 'n'
            , '\r' <$ char 'r'
            , '\t' <$ char 't'
            , unicodeEscape
            ]
    unicodeEscape = do
        _ <- char 'u'
        hexDigits <- countExactly 4 hexDigitParser
        pure (chr (foldl (\acc digitValue -> acc * 16 + digitToInt digitValue) 0 hexDigits))

jsonNumberParser :: ReadP Double
jsonNumberParser = do
    sign <- option "" (string "-")
    whole <- ifZeroPrefixedNumber
    fractional <- option "" ((:) <$> char '.' <*> munch1 isDigit)
    exponentPart <- option "" exponentParser
    pure (read (sign ++ whole ++ fractional ++ exponentPart))
  where
    ifZeroPrefixedNumber =
        (string "0")
            <|> ((:) <$> satisfyDigitOneToNine <*> manyDigitParser)
    exponentParser = do
        exponentMarker <- choice [char 'e', char 'E']
        exponentSign <- option "" (string "+" <|> string "-")
        exponentDigits <- munch1 isDigit
        pure (exponentMarker : exponentSign ++ exponentDigits)

manyDigitParser :: ReadP String
manyDigitParser =
    many (satisfy isDigit)

satisfyDigitOneToNine :: ReadP Char
satisfyDigitOneToNine =
    satisfy (`elem` ['1' .. '9'])

hexDigitParser :: ReadP Char
hexDigitParser =
    satisfy (`elem` (['0' .. '9'] ++ ['a' .. 'f'] ++ ['A' .. 'F']))

satisfyNot :: [Char] -> ReadP Char
satisfyNot disallowed =
    satisfy (`notElem` disallowed)

countExactly :: Int -> ReadP a -> ReadP [a]
countExactly n parser
    | n <= 0 = pure []
    | otherwise = (:) <$> parser <*> countExactly (n - 1) parser

escapeJsonChar :: Char -> String
escapeJsonChar charValue =
    case charValue of
        '"' -> "\\\""
        '\\' -> "\\\\"
        '\b' -> "\\b"
        '\f' -> "\\f"
        '\n' -> "\\n"
        '\r' -> "\\r"
        '\t' -> "\\t"
        _ | ord charValue < 0x20 -> "\\u" ++ padHex (showHex (ord charValue) "")
        _ -> [charValue]

padHex :: String -> String
padHex hexDigits =
    replicate (4 - length hexDigits) '0' ++ hexDigits

isWholeNumber :: Double -> Bool
isWholeNumber numberValue =
    numberValue == fromIntegral (round numberValue :: Integer)
