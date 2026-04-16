module RespProtocol
    ( description
    , RespValue(..)
    , RespDecodeError(..)
    , RespEncodeError(..)
    , decode
    , decodeAll
    , encode
    , encodeSimpleString
    , encodeError
    , encodeInteger
    , encodeBulkString
    , encodeArray
    ) where

import qualified Data.ByteString as BS
import qualified Data.ByteString.Char8 as BC

description :: String
description = "Haskell RESP2 encoder and decoder"

data RespValue
    = RespSimpleString String
    | RespError String
    | RespInteger Integer
    | RespBulkString (Maybe BS.ByteString)
    | RespArray (Maybe [RespValue])
    deriving (Eq, Show)

newtype RespDecodeError = RespDecodeError
    { respDecodeErrorMessage :: String
    }
    deriving (Eq, Show)

newtype RespEncodeError = RespEncodeError
    { respEncodeErrorMessage :: String
    }
    deriving (Eq, Show)

decode :: BS.ByteString -> Either RespDecodeError (Maybe (RespValue, Int))
decode buffer
    | BS.null buffer = Right Nothing
    | otherwise =
        case BS.head buffer of
            43 -> decodeSimpleString buffer
            45 -> decodeErrorString buffer
            58 -> decodeIntegerValue buffer
            36 -> decodeBulkStringValue buffer
            42 -> decodeArrayValue buffer
            _ -> decodeInlineCommand buffer

decodeAll :: BS.ByteString -> Either RespDecodeError ([RespValue], Int)
decodeAll = go 0 []
  where
    go offset values buffer =
        case decode buffer of
            Left decodeError -> Left decodeError
            Right Nothing -> Right (reverse values, offset)
            Right (Just (value, consumed)) ->
                go (offset + consumed) (value : values) (BS.drop consumed buffer)

encode :: RespValue -> Either RespEncodeError BS.ByteString
encode value =
    case value of
        RespSimpleString textValue -> encodeSimpleString textValue
        RespError textValue -> Right (encodeError textValue)
        RespInteger numberValue -> Right (encodeInteger numberValue)
        RespBulkString bytesValue -> Right (encodeBulkString bytesValue)
        RespArray valuesValue -> encodeArray valuesValue

encodeSimpleString :: String -> Either RespEncodeError BS.ByteString
encodeSimpleString textValue
    | any (`elem` ['\r', '\n']) textValue =
        Left
            (RespEncodeError
                ("simple string must not contain carriage return or newline: " ++ show textValue))
    | otherwise =
        Right (BC.pack ('+' : textValue ++ "\r\n"))

encodeError :: String -> BS.ByteString
encodeError textValue = BC.pack ('-' : textValue ++ "\r\n")

encodeInteger :: Integer -> BS.ByteString
encodeInteger numberValue = BC.pack (':' : show numberValue ++ "\r\n")

encodeBulkString :: Maybe BS.ByteString -> BS.ByteString
encodeBulkString maybeBytes =
    case maybeBytes of
        Nothing -> BC.pack "$-1\r\n"
        Just bytesValue ->
            BC.pack ('$' : show (BS.length bytesValue) ++ "\r\n")
                <> bytesValue
                <> BC.pack "\r\n"

encodeArray :: Maybe [RespValue] -> Either RespEncodeError BS.ByteString
encodeArray maybeValues =
    case maybeValues of
        Nothing -> Right (BC.pack "*-1\r\n")
        Just valuesList -> do
            encodedValues <- mapM encode valuesList
            pure (BC.pack ('*' : show (length valuesList) ++ "\r\n") <> BS.concat encodedValues)

decodeSimpleString :: BS.ByteString -> Either RespDecodeError (Maybe (RespValue, Int))
decodeSimpleString buffer =
    case readLine (BS.tail buffer) of
        Nothing -> Right Nothing
        Just (lineBytes, consumed) ->
            Right (Just (RespSimpleString (BC.unpack lineBytes), consumed + 1))

decodeErrorString :: BS.ByteString -> Either RespDecodeError (Maybe (RespValue, Int))
decodeErrorString buffer =
    case readLine (BS.tail buffer) of
        Nothing -> Right Nothing
        Just (lineBytes, consumed) ->
            Right (Just (RespError (BC.unpack lineBytes), consumed + 1))

decodeIntegerValue :: BS.ByteString -> Either RespDecodeError (Maybe (RespValue, Int))
decodeIntegerValue buffer =
    case readLine (BS.tail buffer) of
        Nothing -> Right Nothing
        Just (lineBytes, consumed) ->
            case BC.readInteger lineBytes of
                Just (value, rest) | BS.null rest ->
                    Right (Just (RespInteger value, consumed + 1))
                _ -> Left (RespDecodeError "invalid RESP integer")

decodeBulkStringValue :: BS.ByteString -> Either RespDecodeError (Maybe (RespValue, Int))
decodeBulkStringValue buffer =
    case readLine (BS.tail buffer) of
        Nothing -> Right Nothing
        Just (lineBytes, consumed) ->
            case BC.readInteger lineBytes of
                Just (-1, rest) | BS.null rest ->
                    Right (Just (RespBulkString Nothing, consumed + 1))
                Just (lengthValue, rest)
                    | BS.null rest && lengthValue >= 0 ->
                        let bodyStart = consumed + 1
                            bodyEnd = bodyStart + fromIntegral lengthValue
                            tailEnd = bodyEnd + 2
                         in if BS.length buffer < tailEnd
                                then Right Nothing
                                else if BS.take 2 (BS.drop bodyEnd buffer) /= BC.pack "\r\n"
                                    then Left (RespDecodeError "missing trailing CRLF after bulk string body")
                                    else Right
                                        (Just
                                            ( RespBulkString (Just (BS.take (fromIntegral lengthValue) (BS.drop bodyStart buffer)))
                                            , tailEnd
                                            ))
                _ -> Left (RespDecodeError "invalid RESP bulk string length")

decodeArrayValue :: BS.ByteString -> Either RespDecodeError (Maybe (RespValue, Int))
decodeArrayValue buffer =
    case readLine (BS.tail buffer) of
        Nothing -> Right Nothing
        Just (lineBytes, consumed) ->
            case BC.readInteger lineBytes of
                Just (-1, rest) | BS.null rest ->
                    Right (Just (RespArray Nothing, consumed + 1))
                Just (countValue, rest)
                    | BS.null rest && countValue >= 0 ->
                        decodeArrayItems (fromIntegral countValue) (consumed + 1) []
                _ -> Left (RespDecodeError "invalid RESP array length")
  where
    decodeArrayItems remaining offset values
        | remaining == 0 =
            Right (Just (RespArray (Just (reverse values)), offset))
        | otherwise =
            case decode (BS.drop offset buffer) of
                Left decodeError -> Left decodeError
                Right Nothing -> Right Nothing
                Right (Just (value, used)) ->
                    decodeArrayItems (remaining - 1) (offset + used) (value : values)

decodeInlineCommand :: BS.ByteString -> Either RespDecodeError (Maybe (RespValue, Int))
decodeInlineCommand buffer =
    case readLine buffer of
        Nothing -> Right Nothing
        Just (lineBytes, consumed) ->
            Right
                (Just
                    ( RespArray
                        (Just [RespBulkString (Just token) | token <- BC.words lineBytes])
                    , consumed
                    ))

readLine :: BS.ByteString -> Maybe (BS.ByteString, Int)
readLine buffer = go 0
  where
    bufferLength = BS.length buffer
    go indexValue
        | indexValue + 1 >= bufferLength = Nothing
        | BS.index buffer indexValue == 13
            && BS.index buffer (indexValue + 1) == 10 =
                Just (BS.take indexValue buffer, indexValue + 2)
        | otherwise = go (indexValue + 1)
