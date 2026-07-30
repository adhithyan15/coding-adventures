-- | Pure, bounded HTTP/1 request and response head parsing.
--
-- The parser owns the boundary between untrusted connection bytes and the
-- semantic message types from @http-core@. It performs no socket I/O and never
-- consumes body bytes. Wire grammar and framing decisions fail closed so two
-- recipients cannot silently disagree about where one message ends.
module CodingAdventures.Http1
  ( ParsedRequestHead (..)
  , ParsedResponseHead (..)
  , ResponseContext (..)
  , Http1ParseError (..)
  , version
  , parseRequestHead
  , parseResponseHead
  ) where

import CodingAdventures.HttpCore
  ( BodyKind (..)
  , Header (..)
  , HttpVersion (..)
  , RequestHead (..)
  , ResponseHead (..)
  , parseHttpVersion
  )
import Data.ByteString (ByteString)
import qualified Data.ByteString as Bytes
import qualified Data.ByteString.Char8 as Bytes8
import Data.List (dropWhileEnd)
import Data.Word (Word16, Word8)

-- | Package version shared across implementation languages.
version :: String
version = "0.1.0"

-- | One parsed request head plus the untouched body boundary.
data ParsedRequestHead = ParsedRequestHead
  { parsedRequest :: RequestHead
  , requestBodyOffset :: Int
  , requestBodyKind :: BodyKind
  } deriving (Eq, Show)

-- | One parsed response head plus the untouched body boundary.
--
-- A successful response to CONNECT switches to tunnel bytes immediately after
-- the head. 'responseSwitchesProtocol' distinguishes that transition from an
-- ordinary bodyless response.
data ParsedResponseHead = ParsedResponseHead
  { parsedResponse :: ResponseHead
  , responseBodyOffset :: Int
  , responseBodyKind :: BodyKind
  , responseSwitchesProtocol :: Bool
  } deriving (Eq, Show)

-- | Request facts required to frame the corresponding response safely.
data ResponseContext = ResponseContext
  { contextRequestMethod :: String
  , contextRequestVersion :: HttpVersion
  } deriving (Eq, Show)

-- | Stable, redacted failure classes from the NET04 contract.
--
-- No constructor retains raw wire text. Request targets, credentials, and
-- field values therefore cannot escape when a caller logs an error.
data Http1ParseError
  = IncompleteHead
  | HeadTooLarge
  | LineTooLong
  | TooManyHeaders
  | TooManyTransferCodings
  | InvalidStartLine
  | InvalidVersion
  | InvalidStatusCode
  | InvalidHeaderLine
  | InvalidContentLength
  | InvalidTransferEncoding
  | AmbiguousFraming
  deriving (Eq, Show)

-- | Parse an HTTP/1 request head from strict bytes.
parseRequestHead :: ByteString -> Either Http1ParseError ParsedRequestHead
parseRequestHead input = do
  (linesInHead, bodyOffset) <- splitHeadLines input
  (startLine, headerLines) <- firstLine linesInHead
  (method, target, versionText) <- parseRequestLine startLine
  httpVersion <-
    mapLeft (const InvalidVersion) (parseHttpVersion versionText)
  headers <- parseHeaders headerLines
  bodyKind <- determineRequestBodyKind httpVersion headers
  Right
    ParsedRequestHead
      { parsedRequest =
          RequestHead
            { requestMethod = method
            , requestTarget = target
            , requestVersion = httpVersion
            , requestHeaders = headers
            }
      , requestBodyOffset = bodyOffset
      , requestBodyKind = bodyKind
      }

-- | Parse an HTTP/1 response head using its corresponding request method.
--
-- Response body rules depend on request semantics: HEAD responses never have a
-- message body, and successful CONNECT responses transition to a tunnel.
parseResponseHead
  :: ResponseContext
  -> ByteString
  -> Either Http1ParseError ParsedResponseHead
parseResponseHead context input = do
  (linesInHead, bodyOffset) <- splitHeadLines input
  (statusLine, headerLines) <- firstLine linesInHead
  (versionText, statusCodeText, reason) <- parseStatusLine statusLine
  httpVersion <-
    mapLeft (const InvalidVersion) (parseHttpVersion versionText)
  statusCode <-
    if length statusCodeText /= 3
      then Left InvalidStatusCode
      else
        case parseBoundedDecimal (999 :: Word16) statusCodeText of
          Just parsedStatus
            | parsedStatus >= 100 -> Right parsedStatus
          _ -> Left InvalidStatusCode
  headers <- parseHeaders headerLines
  (bodyKind, switchesProtocol) <-
    determineResponseBodyKind context httpVersion statusCode headers
  Right
    ParsedResponseHead
      { parsedResponse =
          ResponseHead
            { responseVersion = httpVersion
            , responseStatus = statusCode
            , responseReason = reason
            , responseHeaders = headers
            }
      , responseBodyOffset = bodyOffset
      , responseBodyKind = bodyKind
      , responseSwitchesProtocol = switchesProtocol
      }

-- Resource bounds -----------------------------------------------------------

maxHeadBytes :: Int
maxHeadBytes = 65536

maxLineBytes :: Int
maxLineBytes = 8192

maxHeaderCount :: Int
maxHeaderCount = 100

maxTransferCodingCount :: Int
maxTransferCodingCount = 16

-- Head framing --------------------------------------------------------------

-- | Separate bounded head lines without decoding or copying body bytes.
--
-- Leading blank lines are skipped, CRLF and bare LF are accepted, and the
-- returned offset remains relative to the original input.
splitHeadLines :: ByteString -> Either Http1ParseError ([ByteString], Int)
splitHeadLines input = collect 0 False 0 []
  where
    inputLength = Bytes.length input

    collect index sawStart headerCount reversedLines
      | index >= maxHeadBytes =
          if index < inputLength
            then Left HeadTooLarge
            else Left IncompleteHead
      | index >= inputLength = Left IncompleteHead
      | otherwise =
          case nextLine index of
            Left failure -> Left failure
            Right (line, nextIndex)
              | Bytes.null line && not sawStart ->
                  collect nextIndex False 0 []
              | Bytes.null line ->
                  Right (reverse reversedLines, nextIndex)
              | not sawStart ->
                  collect nextIndex True 0 [line]
              | headerCount >= maxHeaderCount ->
                  Left TooManyHeaders
              | otherwise ->
                  collect
                    nextIndex
                    True
                    (headerCount + 1)
                    (line : reversedLines)

    nextLine index =
      let remaining = Bytes.drop index input
          searchBudget =
            min
              (maxLineBytes + 2)
              (maxHeadBytes - index + 1)
          searchWindow = Bytes.take searchBudget remaining
      in case Bytes.elemIndex lineFeed searchWindow of
          Nothing
            | Bytes.length remaining > maxLineBytes + 1 -> Left LineTooLong
            | inputLength > maxHeadBytes -> Left HeadTooLarge
            | otherwise -> Left IncompleteHead
          Just relativeLineEnd ->
            let nextIndex = index + relativeLineEnd + 1
                rawLine = Bytes.take relativeLineEnd remaining
                line = dropTrailingCarriageReturn rawLine
            in if nextIndex > maxHeadBytes
                then Left HeadTooLarge
                else if Bytes.length line > maxLineBytes
                  then Left LineTooLong
                  else Right (line, nextIndex)

carriageReturn :: Word8
carriageReturn = 13

lineFeed :: Word8
lineFeed = 10

dropTrailingCarriageReturn :: ByteString -> ByteString
dropTrailingCarriageReturn input
  | not (Bytes.null input) && Bytes.last input == carriageReturn =
      Bytes.init input
  | otherwise = input

-- Start lines and headers ---------------------------------------------------

firstLine :: [ByteString] -> Either Http1ParseError (ByteString, [ByteString])
firstLine [] = Left InvalidStartLine
firstLine (line : rest) = Right (line, rest)

parseRequestLine
  :: ByteString
  -> Either Http1ParseError (String, String, String)
parseRequestLine input =
  case Bytes8.split ' ' input of
    [methodBytes, targetBytes, versionBytes]
      | not (Bytes.null methodBytes)
      , Bytes.all isTokenByte methodBytes
      , not (Bytes.null targetBytes)
      , Bytes.all isRequestTargetByte targetBytes
      , not (Bytes.null versionBytes) ->
          Right
            ( bytesToString methodBytes
            , bytesToString targetBytes
            , bytesToString versionBytes
            )
    _ -> Left InvalidStartLine

parseStatusLine
  :: ByteString
  -> Either Http1ParseError (String, String, String)
parseStatusLine input =
  case splitByte space input of
    (versionBytes, Just afterVersion)
      | not (Bytes.null versionBytes) ->
          case splitByte space afterVersion of
            (statusBytes, maybeReason)
              | not (Bytes.null statusBytes)
              , maybe True validReasonPhrase maybeReason ->
                  Right
                    ( bytesToString versionBytes
                    , bytesToString statusBytes
                    , maybe "" bytesToString maybeReason
                    )
            _ -> Left InvalidStartLine
    _ -> Left InvalidStartLine

validReasonPhrase :: ByteString -> Bool
validReasonPhrase = Bytes.all isFieldValueByte

parseHeaders :: [ByteString] -> Either Http1ParseError [Header]
parseHeaders = traverse parseHeader

parseHeader :: ByteString -> Either Http1ParseError Header
parseHeader input =
  case splitByte colon input of
    (nameBytes, Just rawValue)
      | not (Bytes.null nameBytes)
      , Bytes.all isTokenByte nameBytes
      , Bytes.all isFieldValueByte rawValue ->
          Right
            Header
              { headerName = bytesToString nameBytes
              , headerValue = trimOws (bytesToString rawValue)
              }
    _ -> Left InvalidHeaderLine

bytesToString :: ByteString -> String
bytesToString = Bytes8.unpack

splitByte :: Word8 -> ByteString -> (ByteString, Maybe ByteString)
splitByte delimiter input =
  case Bytes.elemIndex delimiter input of
    Nothing -> (input, Nothing)
    Just index ->
      ( Bytes.take index input
      , Just (Bytes.drop (index + 1) input)
      )

trimOws :: String -> String
trimOws = dropWhileEnd isOws . dropWhile isOws

isOws :: Char -> Bool
isOws character = character == ' ' || character == '\t'

space :: Word8
space = 32

colon :: Word8
colon = 58

isAsciiDigitByte :: Word8 -> Bool
isAsciiDigitByte byte = byte >= 48 && byte <= 57

isRequestTargetByte :: Word8 -> Bool
isRequestTargetByte byte = byte > 32 && byte /= 127

isFieldValueByte :: Word8 -> Bool
isFieldValueByte byte =
  byte == 9
    || byte == 32
    || byte >= 33 && byte /= 127

isTokenByte :: Word8 -> Bool
isTokenByte byte =
  isAsciiDigitByte byte
    || byte >= 65 && byte <= 90
    || byte >= 97 && byte <= 122
    || byte `elem` tokenPunctuation
  where
    tokenPunctuation =
      [ 33 -- !
      , 35 -- #
      , 36 -- $
      , 37 -- %
      , 38 -- &
      , 39 -- '
      , 42 -- *
      , 43 -- +
      , 45 -- -
      , 46 -- .
      , 94 -- ^
      , 95 -- _
      , 96 -- `
      , 124 -- |
      , 126 -- ~
      ]

-- Body framing --------------------------------------------------------------

determineRequestBodyKind
  :: HttpVersion
  -> [Header]
  -> Either Http1ParseError BodyKind
determineRequestBodyKind httpVersion headers
  | hasTransferEncoding && hasContentLength = Left AmbiguousFraming
  | hasTransferEncoding = do
      codings <- transferCodings headers
      if not (supportsTransferEncoding httpVersion)
          || not (validFinalChunked codings)
        then Left InvalidTransferEncoding
        else Right Chunked
  | otherwise = bodyKindFromContentLength NoBody headers
  where
    hasTransferEncoding = hasHeader headers "Transfer-Encoding"
    hasContentLength = hasHeader headers "Content-Length"

determineResponseBodyKind
  :: ResponseContext
  -> HttpVersion
  -> Word16
  -> [Header]
  -> Either Http1ParseError (BodyKind, Bool)
determineResponseBodyKind context parsedResponseVersion statusCode headers
  | contextRequestMethod context == "HEAD" = Right (NoBody, False)
  | contextRequestMethod context == "CONNECT"
      && statusCode >= 200
      && statusCode < 300 =
      Right (NoBody, True)
  | statusCode >= 100 && statusCode < 200 = Right (NoBody, False)
  | statusCode == 204 || statusCode == 304 = Right (NoBody, False)
  | hasTransferEncoding && hasContentLength = Left AmbiguousFraming
  | hasTransferEncoding = do
      codings <- transferCodings headers
      if not (supportsTransferEncoding (contextRequestVersion context))
          || not (supportsTransferEncoding parsedResponseVersion)
          || containsNonFinalOrRepeatedChunked codings
        then Left InvalidTransferEncoding
        else
          Right
            ( if not (null codings) && asciiEqual (last codings) "chunked"
                then Chunked
                else UntilEof
            , False
            )
  | otherwise = do
      bodyKind <- bodyKindFromContentLength UntilEof headers
      Right (bodyKind, False)
  where
    hasTransferEncoding = hasHeader headers "Transfer-Encoding"
    hasContentLength = hasHeader headers "Content-Length"

bodyKindFromContentLength
  :: BodyKind
  -> [Header]
  -> Either Http1ParseError BodyKind
bodyKindFromContentLength absentKind headers = do
  declaredLength <- declaredContentLength headers
  Right
    (case declaredLength of
      Nothing -> absentKind
      Just 0 -> NoBody
      Just lengthValue -> ContentLength lengthValue)

declaredContentLength :: [Header] -> Either Http1ParseError (Maybe Int)
declaredContentLength headers =
  case headerValues headers "Content-Length" of
    [] -> Right Nothing
    rawValues -> do
      let pieces = concatMap (splitOn ',') rawValues
      parsedValues <- traverse parseOne pieces
      case parsedValues of
        [] -> Left InvalidContentLength
        firstValue : rest
          | all (== firstValue) rest -> Right (Just firstValue)
          | otherwise -> Left InvalidContentLength
  where
    parseOne rawValue =
      case parseBoundedDecimal (maxBound :: Int) (trimOws rawValue) of
        Nothing -> Left InvalidContentLength
        Just parsedValue -> Right parsedValue

transferCodings :: [Header] -> Either Http1ParseError [String]
transferCodings headers =
  let pieces =
        concatMap
          (splitOn ',')
          (headerValues headers "Transfer-Encoding")
  in if length pieces > maxTransferCodingCount
      then Left TooManyTransferCodings
      else traverse parseCoding pieces
  where
    parseCoding rawCoding =
      let codingName = trimOws rawCoding
      in if null codingName
          || ';' `elem` codingName
          || not (all isTokenCharacter codingName)
          then Left InvalidTransferEncoding
          else Right codingName

validFinalChunked :: [String] -> Bool
validFinalChunked codings =
  not (null codings)
    && asciiEqual (last codings) "chunked"
    && countChunked codings == 1

containsNonFinalOrRepeatedChunked :: [String] -> Bool
containsNonFinalOrRepeatedChunked codings =
  null codings
    || countChunked codings > 1
    || (countChunked codings == 1 && not (asciiEqual (last codings) "chunked"))

countChunked :: [String] -> Int
countChunked = length . filter (`asciiEqual` "chunked")

supportsTransferEncoding :: HttpVersion -> Bool
supportsTransferEncoding httpVersion =
  versionMajor httpVersion > 1
    || (versionMajor httpVersion == 1 && versionMinor httpVersion >= 1)

hasHeader :: [Header] -> String -> Bool
hasHeader headers name = not (null (headerValues headers name))

headerValues :: [Header] -> String -> [String]
headerValues headers name =
  [ headerValue header
  | header <- headers
  , asciiEqual (headerName header) name
  ]

-- Bounded protocol helpers --------------------------------------------------

parseBoundedDecimal :: Integral value => value -> String -> Maybe value
parseBoundedDecimal limit text
  | null text = Nothing
  | otherwise = go 0 text
  where
    go accumulator [] = Just accumulator
    go accumulator (character : rest)
      | not (isAsciiDigit character) = Nothing
      | accumulator > (limit - digit) `div` 10 = Nothing
      | otherwise = go (accumulator * 10 + digit) rest
      where
        digit = fromIntegral (fromEnum character - fromEnum '0')

isAsciiDigit :: Char -> Bool
isAsciiDigit character = character >= '0' && character <= '9'

isTokenCharacter :: Char -> Bool
isTokenCharacter character =
  fromEnum character <= 255
    && isTokenByte (fromIntegral (fromEnum character))

asciiEqual :: String -> String -> Bool
asciiEqual left right = map lowerAscii left == map lowerAscii right
  where
    lowerAscii character
      | character >= 'A' && character <= 'Z' =
          toEnum (fromEnum character + fromEnum 'a' - fromEnum 'A')
      | otherwise = character

splitOn :: Char -> String -> [String]
splitOn delimiter input =
  case break (== delimiter) input of
    (piece, []) -> [piece]
    (piece, _ : rest) -> piece : splitOn delimiter rest

mapLeft :: (leftA -> leftB) -> Either leftA right -> Either leftB right
mapLeft transform result =
  case result of
    Left value -> Left (transform value)
    Right value -> Right value
