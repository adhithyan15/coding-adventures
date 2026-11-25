-- | Replays the Go fuzz engine's 820 oracle vectors (VER06 8) against THIS
-- package's mutators.  The table is parsed from the committed Go file (no copy);
-- a row that cannot be parsed, or an op the engine does not know, is a failure.
module Vectors
  ( Vector (..)
  , parseVectors
  , runVector
  , vectorOps
  ) where

import qualified Data.ByteString as B
import qualified Data.ByteString.Char8 as BC
import Data.List (stripPrefix)
import Data.Maybe (fromMaybe)

import CodingAdventures.TestBootstrap.Fuzz.Mutate
import CodingAdventures.TestBootstrap.Util (hexLower, unhex)

data Vector = Vector {vOp :: String, vArgs :: [String], vOut :: String}
  deriving (Show)

vectorOps :: [String]
vectorOps = ["bit_flip", "byte_flip", "arithmetic", "boundary", "block_delete", "block_insert", "block_duplicate", "splice", "sequence"]

-- | The rows between @var referenceVectors = []refVector{@ and the closing brace.
-- A row is @\\t{"op", []string{"a", "b"}, "out"},@: its quoted strings are the op,
-- the arguments and the output.
parseVectors :: B.ByteString -> Either String [Vector]
parseVectors raw = case dropWhile (/= "var referenceVectors = []refVector{") (lines (BC.unpack raw)) of
  [] -> Left "the vector table moved"
  (_ : rest) -> mapM row (zip [0 :: Int ..] (takeWhile (/= "}") rest))
  where
    row (i, line) = case stripPrefix "\t{\"" line of
      Just _ | Just _ <- stripSuffix "\"}," line ->
        let quoted = [q | (k, q) <- zip [0 :: Int ..] (splitOn '"' line), odd k]
         in case quoted of
              (op : more@(_ : _)) -> Right (Vector op (init more) (last more))
              _ -> Left ("unparseable vector row " ++ show i)
      _ -> Left ("unparseable vector row " ++ show i ++ ": " ++ line)
    stripSuffix suf s = fmap reverse (stripPrefix (reverse suf) (reverse s))
    splitOn c s = case break (== c) s of
      (a, []) -> [a]
      (a, _ : t) -> a : splitOn c t

-- | Evaluates one vector; @Left@ says what went wrong.
runVector :: Vector -> IO (Either String ())
runVector v@(Vector op args _) = case checkTokens op args of
  Just e -> pure (Left e)
  Nothing -> runChecked v

-- | Validates every token BEFORE it is converted: integers are at most 40 decimal
-- digits (optional minus), hex tokens are lowercase even-length hex.  A bad token is
-- a clear failure, never a huge @read@ or a garbage decode.
checkTokens :: String -> [String] -> Maybe String
checkTokens op args = case lookup op layout of
  Nothing -> Nothing
  Just kinds
    | length kinds /= length args -> Just (op ++ ": wrong arity " ++ show (length args))
    | otherwise -> case [e | (k, a) <- zip kinds args, Just e <- [bad k a]] of
        (e : _) -> Just (op ++ ": " ++ e)
        [] -> Nothing
  where
    bad 'i' a = if numeric a then Nothing else Just ("non-numeric or over-long integer token " ++ show (take 50 a))
    bad 'h' a = if hexOk a then Nothing else Just ("bad hex token " ++ show (take 50 a))
    bad _ _ = Nothing
    numeric a = case a of
      '-' : d -> digits d
      d -> digits d
    digits d = not (null d) && length d <= 40 && all (`elem` ("0123456789" :: String)) d
    hexOk a = even (length a) && all (`elem` ("0123456789abcdef" :: String)) a
    layout =
      [ ("bit_flip", "hii"), ("byte_flip", "hii"), ("arithmetic", "hiiie"), ("boundary", "hiiie")
      , ("block_delete", "hii"), ("block_insert", "hih"), ("block_duplicate", "hiii")
      , ("splice", "hhii"), ("sequence", "hiii") ]

runChecked :: Vector -> IO (Either String ())
runChecked (Vector op args out) = do
  let h = unhex . BC.pack
      i :: String -> Integer
      i = read
      endian s = fromMaybe Little (parseEndian s)
      check got = pure (if hexLower got == out then Right () else Left (op ++ " " ++ show args ++ ": got " ++ hexLower got ++ ", want " ++ out))
  case (op, args) of
    ("bit_flip", [d, o, c]) -> check (bitFlip (h d) (i o) (i c))
    ("byte_flip", [d, o, c]) -> check (byteFlip (h d) (i o) (i c))
    ("arithmetic", [d, o, dl, w, e]) -> check (byteArithmetic (h d) (i o) (i dl) (i w) (endian e))
    ("boundary", [d, o, w, v, e]) -> check (boundaryInsert (h d) (i o) (i w) (i v) (endian e))
    ("block_delete", [d, o, l]) -> check (blockDelete (h d) (i o) (i l))
    ("block_insert", [d, o, ins]) -> check (blockInsert (h d) (i o) (h ins))
    ("block_duplicate", [d, s, l, t]) -> check (blockDuplicate (h d) (i s) (i l) (i t))
    ("splice", [a, b, ts, ds]) -> check (splice (h a) (h b) (i ts) (i ds))
    ("sequence", [d, sd, st, ml]) -> mutateSequence (h d) (i sd) (i st) (fromInteger (i ml)) >>= check
    _ -> pure (Left ("unknown vector op or arity: " ++ op))
