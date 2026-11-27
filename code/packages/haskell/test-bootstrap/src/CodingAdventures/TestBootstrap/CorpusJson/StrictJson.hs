-- | A small, STRICT JSON reader (VER05n 8).
--
-- A conformance fixture is DATA, and a lenient parser quietly papers over
-- exactly the corruption a conformance check exists to notice (for instance a
-- parser that keeps the LAST of two duplicate keys).  So the corpus loader owns
-- its parser.  Refused:
--
-- * duplicate object keys, compared by their DECODED bytes (@"a"@ and
--   @"\\u0061"@ are the same key), and trailing data after the document;
-- * nesting deeper than 64 containers (refused before recursing, so a hostile
--   million-deep list cannot exhaust the stack);
-- * documents over 8 MiB or over 200,000 values (object keys count);
-- * invalid UTF-8 (overlong forms, surrogates, above U+10FFFF) and a BOM;
-- * raw control characters in strings, lone or reversed surrogate escapes,
--   leading zeros, @+1@, @NaN@, comments, trailing commas.
--
-- Numbers are kept as their literal TEXT ('JNumber'): a 30-digit integer must
-- not lose its low digits to a 'Double'.  Strings are kept as UTF-8 bytes.
module CodingAdventures.TestBootstrap.CorpusJson.StrictJson
  ( Json (..)
  , JsonError (..)
  , parse
  , maxJsonDepth
  , maxJsonBytes
  , maxJsonNodes
  ) where

import Data.Bits (shiftL, (.&.), (.|.))
import qualified Data.ByteString as B
import qualified Data.ByteString.Builder as BB
import qualified Data.ByteString.Char8 as BC
import qualified Data.ByteString.Lazy as BL
import Data.Char (chr)
import qualified Data.Set as Set
import Data.Word (Word8)

import CodingAdventures.TestBootstrap.Util (validUtf8)

data Json
  = JNull
  | JBool !Bool
  | JNumber !B.ByteString
  | JString !B.ByteString
  | JArray [Json]
  | JObject [(B.ByteString, Json)]
  deriving (Eq, Show)

newtype JsonError = JsonError String deriving (Eq, Show)

maxJsonDepth, maxJsonBytes, maxJsonNodes :: Int
maxJsonDepth = 64
maxJsonBytes = 8 * 1024 * 1024
maxJsonNodes = 200000

-- | The parser state: input, position, values counted so far.
newtype P a = P {runP :: B.ByteString -> Int -> Int -> Either JsonError (a, Int, Int)}

instance Functor P where
  fmap f (P g) = P (\s p n -> fmap (\(a, p', n') -> (f a, p', n')) (g s p n))

instance Applicative P where
  pure x = P (\_ p n -> Right (x, p, n))
  P f <*> P x = P (\s p n -> case f s p n of
    Left e -> Left e
    Right (g, p', n') -> case x s p' n' of
      Left e -> Left e
      Right (a, p'', n'') -> Right (g a, p'', n''))

instance Monad P where
  P m >>= k = P (\s p n -> case m s p n of
    Left e -> Left e
    Right (a, p', n') -> runP (k a) s p' n')

failAt :: String -> P a
failAt msg = P (\_ p _ -> Left (JsonError (msg ++ " at byte " ++ show p)))

peek :: P (Maybe Word8)
peek = P (\s p n -> Right (if p < B.length s then Just (B.index s p) else Nothing, p, n))

peekAt :: Int -> P (Maybe Word8)
peekAt off = P (\s p n -> let q = p + off in Right (if q < B.length s then Just (B.index s q) else Nothing, p, n))

advance :: Int -> P ()
advance k = P (\_ p n -> Right ((), p + k, n))

position :: P Int
position = P (\_ p n -> Right (p, p, n))

slice :: Int -> Int -> P B.ByteString
slice start end = P (\s p n -> Right (B.take (end - start) (B.drop start s), p, n))

countNode :: P ()
countNode = P (\_ p n -> if n + 1 > maxJsonNodes then Left (JsonError ("more than " ++ show maxJsonNodes ++ " values at byte " ++ show p)) else Right ((), p, n + 1))

-- | Strictly parses one JSON document.
parse :: B.ByteString -> Either JsonError Json
parse input
  | B.length input > maxJsonBytes = Left (JsonError ("document of " ++ show (B.length input) ++ " bytes exceeds " ++ show maxJsonBytes))
  | B.isPrefixOf (B.pack [0xEF, 0xBB, 0xBF]) input = Left (JsonError "byte-order mark")
  | not (validUtf8 input) = Left (JsonError "document is not valid UTF-8")
  | otherwise = case runP document input 0 0 of
      Left e -> Left e
      Right (v, _, _) -> Right v
  where
    document = do
      skipWs
      v <- value 0
      skipWs
      end <- atEnd
      if end then pure v else failAt "trailing data after the document"

atEnd :: P Bool
atEnd = P (\s p n -> Right (p >= B.length s, p, n))

skipWs :: P ()
skipWs = do
  c <- peek
  case c of
    Just w | w == 0x20 || w == 0x09 || w == 0x0a || w == 0x0d -> advance 1 >> skipWs
    _ -> pure ()

value :: Int -> P Json
value depth = do
  countNode
  c <- peek
  case c of
    Just 0x7b -> object depth
    Just 0x5b -> array depth
    Just 0x22 -> JString <$> str
    Just 0x74 -> literal "true" (JBool True)
    Just 0x66 -> literal "false" (JBool False)
    Just 0x6e -> literal "null" JNull
    Just w | w == 0x2d || isDigit w -> number
    Nothing -> failAt "unexpected end of input"
    Just w -> failAt ("unexpected byte " ++ show w)

literal :: String -> Json -> P Json
literal word v = do
  p <- position
  s <- slice p (p + length word)
  if s == BC.pack word then advance (length word) >> pure v else failAt "invalid literal"

array :: Int -> P Json
array depth = do
  if depth + 1 > maxJsonDepth then failAt ("nesting deeper than " ++ show maxJsonDepth) else pure ()
  advance 1
  skipWs
  c <- peek
  if c == Just 0x5d
    then advance 1 >> pure (JArray [])
    else JArray <$> items []
  where
    items acc = do
      skipWs
      v <- value (depth + 1)
      skipWs
      c <- peek
      case c of
        Just 0x2c -> advance 1 >> items (v : acc)
        Just 0x5d -> advance 1 >> pure (reverse (v : acc))
        _ -> failAt "expected , or ]"

object :: Int -> P Json
object depth = do
  if depth + 1 > maxJsonDepth then failAt ("nesting deeper than " ++ show maxJsonDepth) else pure ()
  advance 1
  skipWs
  c <- peek
  if c == Just 0x7d
    then advance 1 >> pure (JObject [])
    else JObject <$> members Set.empty []
  where
    members seen acc = do
      skipWs
      k0 <- peek
      if k0 /= Just 0x22 then failAt "expected a string key" else pure ()
      countNode
      key <- str
      if Set.member key seen then failAt "duplicate key" else pure ()
      skipWs
      colon <- peek
      if colon /= Just 0x3a then failAt "expected :" else advance 1
      skipWs
      v <- value (depth + 1)
      skipWs
      c <- peek
      let acc' = (key, v) : acc
          seen' = Set.insert key seen
      case c of
        Just 0x2c -> advance 1 >> members seen' acc'
        Just 0x7d -> advance 1 >> pure (reverse acc')
        _ -> failAt "expected , or }"

isDigit :: Word8 -> Bool
isDigit w = w >= 0x30 && w <= 0x39

number :: P Json
number = do
  start <- position
  c0 <- peek
  if c0 == Just 0x2d then advance 1 else pure ()
  c1 <- peek
  case c1 of
    Just 0x30 -> do
      advance 1
      c2 <- peek
      case c2 of
        Just w | isDigit w -> failAt "leading zero"
        _ -> pure ()
    Just w | isDigit w -> digits
    _ -> failAt "invalid number"
  c3 <- peek
  if c3 == Just 0x2e
    then do
      advance 1
      needDigit "digits required after ."
      digits
    else pure ()
  c4 <- peek
  if c4 == Just 0x65 || c4 == Just 0x45
    then do
      advance 1
      c5 <- peek
      if c5 == Just 0x2b || c5 == Just 0x2d then advance 1 else pure ()
      needDigit "digits required in exponent"
      digits
    else pure ()
  end <- position
  JNumber <$> slice start end
  where
    digits = do
      c <- peek
      case c of
        Just w | isDigit w -> advance 1 >> digits
        _ -> pure ()
    needDigit msg = do
      c <- peek
      case c of
        Just w | isDigit w -> pure ()
        _ -> failAt msg

-- | A string literal, decoded to UTF-8 bytes.
str :: P B.ByteString
str = advance 1 >> go mempty
  where
    go acc = do
      c <- peek
      case c of
        Nothing -> failAt "unterminated string"
        Just 0x22 -> advance 1 >> pure (BL.toStrict (BB.toLazyByteString acc))
        Just w
          | w < 0x20 -> failAt "raw control character in string"
          | w == 0x5c -> advance 1 >> escape >>= \b -> go (acc <> b)
          | otherwise -> do
              start <- position
              runLen
              end <- position
              chunk <- slice start end
              go (acc <> BB.byteString chunk)
    runLen = do
      c <- peek
      case c of
        Just w | w /= 0x22 && w /= 0x5c && w >= 0x20 -> advance 1 >> runLen
        _ -> pure ()

escape :: P BB.Builder
escape = do
  c <- peek
  case c of
    Nothing -> failAt "unterminated escape"
    Just w -> do
      advance 1
      case w of
        0x22 -> pure (BB.char7 '"')
        0x5c -> pure (BB.char7 '\\')
        0x2f -> pure (BB.char7 '/')
        0x62 -> pure (BB.char7 '\b')
        0x66 -> pure (BB.char7 '\f')
        0x6e -> pure (BB.char7 '\n')
        0x72 -> pure (BB.char7 '\r')
        0x74 -> pure (BB.char7 '\t')
        0x75 -> unicode
        _ -> failAt "invalid escape"

unicode :: P BB.Builder
unicode = do
  unit <- hex4
  if unit >= 0xD800 && unit <= 0xDBFF
    then do
      a <- peek
      b <- peekAt 1
      if a /= Just 0x5c || b /= Just 0x75 then failAt "lone high surrogate" else advance 2
      low <- hex4
      if low < 0xDC00 || low > 0xDFFF
        then failAt "invalid surrogate pair"
        else pure (BB.charUtf8 (chr (0x10000 + ((unit - 0xD800) `shiftL` 10) + (low - 0xDC00))))
    else
      if unit >= 0xDC00 && unit <= 0xDFFF
        then failAt "lone low surrogate"
        else pure (BB.charUtf8 (chr unit))

hex4 :: P Int
hex4 = do
  p <- position
  t <- slice p (p + 4)
  if B.length t == 4 && B.all isHex t
    then advance 4 >> pure (B.foldl' (\acc w -> (acc `shiftL` 4) .|. hexVal w) 0 t)
    else failAt "invalid \\u escape"
  where
    isHex w = isDigit w || (w >= 0x41 && w <= 0x46) || (w >= 0x61 && w <= 0x66)
    hexVal :: Word8 -> Int
    hexVal w
      | isDigit w = fromIntegral w - 0x30
      | w >= 0x61 = fromIntegral w - 0x61 + 10
      | otherwise = fromIntegral (w .&. 0xff) - 0x41 + 10
