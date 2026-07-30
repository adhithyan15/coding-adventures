-- | Pure HTTP/1 request and response head parsing.
--
-- An HTTP/1 message crosses two conceptual boundaries:
--
-- @
-- raw connection bytes -> start line and ordered headers -> body reader
-- @
--
-- This module owns the middle boundary. It parses a complete head already
-- present in memory, returns semantic 'RequestHead' or 'ResponseHead' values
-- from @http-core@, and tells the caller both where the body begins and how it
-- is framed. It performs no socket I/O and never consumes body bytes.
module CodingAdventures.Http1
  ( ParsedRequestHead (..)
  , ParsedResponseHead (..)
  , Http1ParseError (..)
  , version
  , parseRequestHead
  , parseResponseHead
  ) where

import CodingAdventures.HttpCore
  ( BodyKind (..)
  , Header (..)
  , RequestHead (..)
  , ResponseHead (..)
  , findHeader
  , parseContentLength
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
data ParsedResponseHead = ParsedResponseHead
  { parsedResponse :: ResponseHead
  , responseBodyOffset :: Int
  , responseBodyKind :: BodyKind
  } deriving (Eq, Show)

-- | Stable failure classes from the NET04 contract.
--
-- The offending start line, token, or header is retained to make malformed
-- fixture failures explainable. Payload bytes are never included because the
-- parser does not inspect them.
data Http1ParseError
  = IncompleteHead
  | InvalidStartLine String
  | InvalidVersion String
  | InvalidStatusCode String
  | InvalidHeaderLine String
  | InvalidContentLength String
  deriving (Eq, Show)

-- | Parse an HTTP/1 request head from strict bytes.
parseRequestHead :: ByteString -> Either Http1ParseError ParsedRequestHead
parseRequestHead input = do
  (linesInHead, bodyOffset) <- splitHeadLines input
  (startLine, headerLines) <- firstLine linesInHead
  let startText = bytesToString startLine
  (method, target, versionText) <-
    case words startText of
      [parsedMethod, parsedTarget, parsedVersion] ->
        Right (parsedMethod, parsedTarget, parsedVersion)
      _ -> Left (InvalidStartLine startText)
  httpVersion <-
    mapLeft (const (InvalidVersion versionText)) (parseHttpVersion versionText)
  headers <- parseHeaders headerLines
  bodyKind <- determineRequestBodyKind headers
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

-- | Parse an HTTP/1 response head from strict bytes.
parseResponseHead :: ByteString -> Either Http1ParseError ParsedResponseHead
parseResponseHead input = do
  (linesInHead, bodyOffset) <- splitHeadLines input
  (statusLine, headerLines) <- firstLine linesInHead
  let statusText = bytesToString statusLine
  (versionText, statusCodeText, reason) <-
    case words statusText of
      parsedVersion : parsedStatus : reasonWords ->
        Right (parsedVersion, parsedStatus, unwords reasonWords)
      _ -> Left (InvalidStartLine statusText)
  httpVersion <-
    mapLeft (const (InvalidVersion versionText)) (parseHttpVersion versionText)
  statusCode <-
    case parseBoundedDecimal (maxBound :: Word16) statusCodeText of
      Nothing -> Left (InvalidStatusCode statusCodeText)
      Just parsedStatus -> Right parsedStatus
  headers <- parseHeaders headerLines
  bodyKind <- determineResponseBodyKind statusCode headers
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
      }

-- Head framing --------------------------------------------------------------

-- | Separate head lines without decoding or copying the body.
--
-- The returned offset is relative to the original input, so leading blank
-- lines remain part of the consumed head. Both CRLF and bare LF are accepted.
splitHeadLines :: ByteString -> Either Http1ParseError ([ByteString], Int)
splitHeadLines input = collect (skipLeadingBlankLines 0) []
  where
    inputLength = Bytes.length input

    skipLeadingBlankLines index
      | index >= inputLength = index
      | Bytes.isPrefixOf crlf (Bytes.drop index input) =
          skipLeadingBlankLines (index + 2)
      | Bytes.index input index == lineFeed =
          skipLeadingBlankLines (index + 1)
      | otherwise = index

    collect index reversedLines
      | index >= inputLength = Left IncompleteHead
      | otherwise =
          case Bytes.elemIndex lineFeed (Bytes.drop index input) of
            Nothing -> Left IncompleteHead
            Just relativeLineEnd ->
              let rawLine =
                    Bytes.take relativeLineEnd (Bytes.drop index input)
                  line = dropTrailingCarriageReturn rawLine
                  nextIndex = index + relativeLineEnd + 1
              in if Bytes.null line
                  then Right (reverse reversedLines, nextIndex)
                  else collect nextIndex (line : reversedLines)

    crlf = Bytes.pack [carriageReturn, lineFeed]

carriageReturn :: Word8
carriageReturn = 13

lineFeed :: Word8
lineFeed = 10

dropTrailingCarriageReturn :: ByteString -> ByteString
dropTrailingCarriageReturn input
  | not (Bytes.null input) && Bytes.last input == carriageReturn =
      Bytes.init input
  | otherwise = input

-- Start lines and headers ----------------------------------------------------

firstLine :: [ByteString] -> Either Http1ParseError (ByteString, [ByteString])
firstLine [] = Left (InvalidStartLine "")
firstLine (line : rest) = Right (line, rest)

parseHeaders :: [ByteString] -> Either Http1ParseError [Header]
parseHeaders = traverse parseHeader

parseHeader :: ByteString -> Either Http1ParseError Header
parseHeader input =
  case break (== ':') text of
    (_, []) -> invalid
    (rawName, _ : rawValue)
      | null name -> invalid
      | otherwise ->
          Right
            Header
              { headerName = name
              , headerValue = trimOws rawValue
              }
      where
        name = trimOws rawName
  where
    text = bytesToString input
    invalid = Left (InvalidHeaderLine text)

bytesToString :: ByteString -> String
bytesToString = Bytes8.unpack

trimOws :: String -> String
trimOws = dropWhileEnd isOws . dropWhile isOws

isOws :: Char -> Bool
isOws character = character == ' ' || character == '\t'

-- Body framing --------------------------------------------------------------

determineRequestBodyKind :: [Header] -> Either Http1ParseError BodyKind
determineRequestBodyKind headers
  | hasChunkedTransferEncoding headers = Right Chunked
  | otherwise = do
      declaredLength <- declaredContentLength headers
      Right
        (case declaredLength of
          Nothing -> NoBody
          Just 0 -> NoBody
          Just lengthValue -> ContentLength lengthValue)

determineResponseBodyKind :: Word16 -> [Header] -> Either Http1ParseError BodyKind
determineResponseBodyKind statusCode headers
  | statusCode >= 100 && statusCode < 200 = Right NoBody
  | statusCode == 204 || statusCode == 304 = Right NoBody
  | hasChunkedTransferEncoding headers = Right Chunked
  | otherwise = do
      declaredLength <- declaredContentLength headers
      Right
        (case declaredLength of
          Nothing -> UntilEof
          Just 0 -> NoBody
          Just lengthValue -> ContentLength lengthValue)

declaredContentLength :: [Header] -> Either Http1ParseError (Maybe Int)
declaredContentLength headers =
  case findHeader headers "Content-Length" of
    Nothing -> Right Nothing
    Just rawValue ->
      case parseContentLength headers of
        Nothing -> Left (InvalidContentLength rawValue)
        Just parsedLength -> Right (Just parsedLength)

hasChunkedTransferEncoding :: [Header] -> Bool
hasChunkedTransferEncoding =
  any headerContainsChunked
  where
    headerContainsChunked header =
      asciiEqual (headerName header) "Transfer-Encoding"
        && any
          (\piece -> asciiEqual (trimOws piece) "chunked")
          (splitOn ',' (headerValue header))

-- Bounded protocol helpers --------------------------------------------------

-- | Fold ASCII digits while checking the target bound before multiplication.
--
-- Using @read@ here would first allocate an arbitrary-precision 'Integer' for
-- a hostile thousand-digit status code. The guarded fold rejects the same
-- input in bounded space and also excludes signs and non-ASCII numerals.
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
