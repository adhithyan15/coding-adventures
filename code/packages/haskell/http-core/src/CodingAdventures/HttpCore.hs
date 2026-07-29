-- | Shared, version-neutral HTTP message types.
--
-- HTTP has two layers:
--
-- @
-- bytes on a connection -> version-specific parser -> semantic message head
-- @
--
-- HTTP/1.x, HTTP/2, and HTTP/3 disagree about bytes on the wire, but callers
-- still need the same ordered headers, version, status, and body-framing hint.
-- This module models that common semantic layer. It deliberately performs no
-- socket I/O and no wire parsing.
module CodingAdventures.HttpCore
  ( Header (..)
  , HttpVersion (..)
  , BodyKind (..)
  , RequestHead (..)
  , ResponseHead (..)
  , RequestTarget (..)
  , RouteSegment (..)
  , RoutePattern (..)
  , version
  , parseHttpVersion
  , renderHttpVersion
  , findHeader
  , parseContentLength
  , parseContentType
  , requestHeader
  , requestContentLength
  , requestContentType
  , responseHeader
  , responseContentLength
  , responseContentType
  , parseRequestTarget
  , queryPairs
  , queryValue
  , requestTargetParts
  , requestPath
  , requestQueryValue
  , splitPathSegments
  , parseRoutePattern
  , matchPath
  , matchTarget
  ) where

import Data.Char (isSpace)
import Data.List (dropWhileEnd, find)
import Data.Word (Word16)
import Text.Read (readMaybe)

-- | Package version shared across implementation languages.
version :: String
version = "0.1.0"

-- | One HTTP header line.
--
-- A list is intentional here: a map would erase duplicate fields such as
-- @Set-Cookie@ and would lose arrival order and original name spelling.
data Header = Header
  { headerName :: String
  , headerValue :: String
  } deriving (Eq, Show)

-- | A future-friendly HTTP version represented as bounded components.
data HttpVersion = HttpVersion
  { versionMajor :: Word16
  , versionMinor :: Word16
  } deriving (Eq, Show)

-- | Parse the exact textual form @HTTP/x.y@.
--
-- Parsing through 'Integer' first makes overflow explicit instead of letting a
-- bounded numeric conversion wrap. Only ASCII decimal digits are accepted.
parseHttpVersion :: String -> Either String HttpVersion
parseHttpVersion input =
  case stripPrefix "HTTP/" input of
    Nothing -> invalid
    Just rest ->
      case splitOnce '.' rest of
        (majorText, Just minorText) ->
          case (parseWord16 majorText, parseWord16 minorText) of
            (Just major, Just minor) ->
              Right HttpVersion
                { versionMajor = major
                , versionMinor = minor
                }
            _ -> invalid
        _ -> invalid
  where
    invalid = Left ("invalid HTTP version: " ++ input)

-- | Render a semantic version back to the stable HTTP marker.
renderHttpVersion :: HttpVersion -> String
renderHttpVersion httpVersion =
  "HTTP/"
    ++ show (versionMajor httpVersion)
    ++ "."
    ++ show (versionMinor httpVersion)

-- | Tell the caller how to consume the payload after parsing the head.
data BodyKind
  = NoBody
  | ContentLength Int
  | UntilEof
  | Chunked
  deriving (Eq, Show)

-- | The semantic fields known after parsing a request head.
data RequestHead = RequestHead
  { requestMethod :: String
  , requestTarget :: String
  , requestVersion :: HttpVersion
  , requestHeaders :: [Header]
  } deriving (Eq, Show)

-- | The semantic fields known after parsing a response head.
data ResponseHead = ResponseHead
  { responseVersion :: HttpVersion
  , responseStatus :: Word16
  , responseReason :: String
  , responseHeaders :: [Header]
  } deriving (Eq, Show)

-- | Return the first matching value using ASCII case-insensitive comparison.
--
-- HTTP field names are ASCII. Restricting the fold to @A-Z@ avoids quietly
-- applying locale or Unicode case rules to a protocol token.
findHeader :: [Header] -> String -> Maybe String
findHeader headers name =
  headerValue <$> find matches headers
  where
    matches header =
      asciiCaseFold (headerName header) == asciiCaseFold name

-- | Parse a non-negative decimal Content-Length that fits in 'Int'.
parseContentLength :: [Header] -> Maybe Int
parseContentLength headers = do
  value <- findHeader headers "Content-Length"
  if null value || not (all isAsciiDigit value)
    then Nothing
    else do
      parsed <- readMaybe value :: Maybe Integer
      if parsed > toInteger (maxBound :: Int)
        then Nothing
        else Just (fromInteger parsed)

-- | Split Content-Type into media type and optional charset.
--
-- Unknown parameters remain intentionally ignored. Charset matching is ASCII
-- case-insensitive, surrounding whitespace is trimmed, and one pair of
-- surrounding double quotes is removed from the charset value.
parseContentType :: [Header] -> Maybe (String, Maybe String)
parseContentType headers = do
  value <- findHeader headers "Content-Type"
  case map trim (splitOn ';' value) of
    [] -> Nothing
    mediaType : parameters
      | null mediaType -> Nothing
      | otherwise -> Just (mediaType, firstCharset parameters)

-- | Request-specific convenience wrappers keep application code declarative.
requestHeader :: RequestHead -> String -> Maybe String
requestHeader request name = findHeader (requestHeaders request) name

requestContentLength :: RequestHead -> Maybe Int
requestContentLength = parseContentLength . requestHeaders

requestContentType :: RequestHead -> Maybe (String, Maybe String)
requestContentType = parseContentType . requestHeaders

-- | Response-specific counterparts to the request helpers.
responseHeader :: ResponseHead -> String -> Maybe String
responseHeader response name = findHeader (responseHeaders response) name

responseContentLength :: ResponseHead -> Maybe Int
responseContentLength = parseContentLength . responseHeaders

responseContentType :: ResponseHead -> Maybe (String, Maybe String)
responseContentType = parseContentType . responseHeaders

-- | A non-decoding view of an origin-form request target.
--
-- Query strings stay raw. A caller can therefore apply its own percent-decoder
-- and duplicate-key policy instead of receiving an irreversible interpretation.
data RequestTarget = RequestTarget
  { targetPath :: String
  , targetQuery :: Maybe String
  , targetFragment :: Maybe String
  } deriving (Eq, Show)

-- | Split a request target at its first fragment marker and first query marker.
-- An omitted path normalizes to @/@, matching browser and server expectations.
parseRequestTarget :: String -> RequestTarget
parseRequestTarget input =
  RequestTarget
    { targetPath = if null path then "/" else path
    , targetQuery = query
    , targetFragment = fragment
    }
  where
    (beforeFragment, fragment) = splitOnce '#' input
    (path, query) = splitOnce '?' beforeFragment

-- | Interpret the raw query as ordered @name=value@ pairs.
--
-- Empty ampersand-delimited pieces are skipped. A flag such as @verbose@ has
-- an empty value, and only the first equals sign separates name from value.
queryPairs :: RequestTarget -> [(String, String)]
queryPairs target =
  map toPair nonEmptyPieces
  where
    pieces = maybe [] (splitOn '&') (targetQuery target)
    nonEmptyPieces = filter (not . null) pieces
    toPair piece =
      case splitOnce '=' piece of
        (name, Just value) -> (name, value)
        (name, Nothing) -> (name, "")

-- | Return the first raw query value with the requested name.
queryValue :: RequestTarget -> String -> Maybe String
queryValue target name =
  snd <$> find ((== name) . fst) (queryPairs target)

requestTargetParts :: RequestHead -> RequestTarget
requestTargetParts = parseRequestTarget . requestTarget

requestPath :: RequestHead -> String
requestPath = targetPath . requestTargetParts

requestQueryValue :: RequestHead -> String -> Maybe String
requestQueryValue request = queryValue (requestTargetParts request)

-- | Split a path into its non-empty slash-delimited pieces.
--
-- Leading, trailing, and repeated slashes do not create phantom route
-- segments. Consequently the root path is represented by the empty list.
splitPathSegments :: String -> [String]
splitPathSegments = filter (not . null) . splitOn '/'

-- | A literal route component or a named capture introduced by @:@.
data RouteSegment
  = Literal String
  | Param String
  deriving (Eq, Show)

-- | A parsed path pattern such as @/devices/:id@.
newtype RoutePattern = RoutePattern
  { routeSegments :: [RouteSegment]
  } deriving (Eq, Show)

parseRoutePattern :: String -> RoutePattern
parseRoutePattern patternText =
  RoutePattern (map parseSegment (splitPathSegments patternText))
  where
    parseSegment (':' : name) = Param name
    parseSegment literal = Literal literal

-- | Match a path and return named captures in pattern order.
matchPath :: RoutePattern -> String -> Maybe [(String, String)]
matchPath patternValue path =
  matchSegments (routeSegments patternValue) (splitPathSegments path)
  where
    matchSegments [] [] = Just []
    matchSegments (Literal expected : rest) (actual : actualRest)
      | expected == actual = matchSegments rest actualRest
      | otherwise = Nothing
    matchSegments (Param name : rest) (actual : actualRest) =
      ((name, actual) :) <$> matchSegments rest actualRest
    matchSegments _ _ = Nothing

-- | Match only the path portion of a full request target.
matchTarget :: RoutePattern -> String -> Maybe [(String, String)]
matchTarget patternValue =
  matchPath patternValue . targetPath . parseRequestTarget

-- Private helpers -----------------------------------------------------------

parseWord16 :: String -> Maybe Word16
parseWord16 text
  | null text || not (all isAsciiDigit text) = Nothing
  | otherwise = do
      parsed <- readMaybe text :: Maybe Integer
      if parsed > toInteger (maxBound :: Word16)
        then Nothing
        else Just (fromInteger parsed)

isAsciiDigit :: Char -> Bool
isAsciiDigit character = character >= '0' && character <= '9'

asciiCaseFold :: String -> String
asciiCaseFold = map lowerAscii
  where
    lowerAscii character
      | character >= 'A' && character <= 'Z' =
          toEnum (fromEnum character + fromEnum 'a' - fromEnum 'A')
      | otherwise = character

firstCharset :: [String] -> Maybe String
firstCharset [] = Nothing
firstCharset (parameter : rest) =
  case splitOnce '=' parameter of
    (name, Just value)
      | asciiCaseFold (trim name) == "charset" ->
          Just (stripSurroundingQuotes (trim value))
    _ -> firstCharset rest

stripSurroundingQuotes :: String -> String
stripSurroundingQuotes text
  | length text >= 2 && head text == '"' && last text == '"' =
      init (tail text)
  | otherwise = text

trim :: String -> String
trim = dropWhileEnd isSpace . dropWhile isSpace

stripPrefix :: String -> String -> Maybe String
stripPrefix [] input = Just input
stripPrefix _ [] = Nothing
stripPrefix (expected : expectedRest) (actual : actualRest)
  | expected == actual = stripPrefix expectedRest actualRest
  | otherwise = Nothing

splitOnce :: Char -> String -> (String, Maybe String)
splitOnce delimiter input =
  case break (== delimiter) input of
    (before, []) -> (before, Nothing)
    (before, _ : after) -> (before, Just after)

splitOn :: Char -> String -> [String]
splitOn delimiter input =
  case splitOnce delimiter input of
    (piece, Nothing) -> [piece]
    (piece, Just rest) -> piece : splitOn delimiter rest
