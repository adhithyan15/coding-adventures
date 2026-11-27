-- | A stable, TOTAL text form of a witness (used in messages and the CI
-- digest).
--
-- > VNil               nil                   VBool         true / false
-- > VInt               decimal; above 100,000 bits "<bigint N bits>"
-- > VBytes             0x<lowercase hex>
-- > VText              "quoted"  (escapes: see "...Escape")
-- > VList              [a,b]                 VTuple        (a,b)
-- > failure            <unrenderable>
--
-- Every cap is enforced WHILE the value is traversed and BEFORE the text it
-- would allocate: depth 16, 10,000 nodes, 1 MiB of output, texts and byte
-- strings clipped to the remaining output budget before they are escaped or
-- hex-encoded, integers compared with 2^100000 before they are shown.  The
-- value is consumed lazily, so an infinite list terminates and a tail that
-- throws turns the WHOLE witness into @<unrenderable>@.
module CodingAdventures.TestBootstrap.CorpusJson.Canon
  ( render
  , renderText
  , unrenderable
  , maxDepth
  , maxNodes
  , maxOutput
  , maxBigintBits
  , sameVal
  ) where

import Control.Exception (Exception, throwIO, try)
import Control.Monad (when)
import qualified Data.ByteString as B
import Data.Bits (shiftL)
import Data.IORef
import Data.Text (Text)
import qualified Data.Text as T

import CodingAdventures.TestBootstrap.CorpusJson.Escape
import CodingAdventures.TestBootstrap.CorpusJson.Val
import CodingAdventures.TestBootstrap.Pbt.Safe (tryRecoverable)
import CodingAdventures.TestBootstrap.Util (hexLower, integerBitLength)

maxDepth, maxNodes, maxOutput, maxBigintBits :: Int
maxDepth = 16
maxNodes = 10000
maxOutput = 1048576
maxBigintBits = 100000

unrenderable :: Text
unrenderable = "<unrenderable>"

-- | Thrown (and caught here) to stop early at a cap.
data Truncated = Truncated deriving (Show)

instance Exception Truncated

-- | Renders any 'Canonical' value; never throws (asynchronous exceptions are
-- rethrown, they are not failures of the value).
renderText :: Canonical a => a -> IO Text
renderText = render . toVal

render :: Val -> IO Text
render v = do
  chunks <- newIORef ([] :: [Text])
  nodes <- newIORef (0 :: Int)
  size <- newIORef (0 :: Int)
  let append t = do
        s <- readIORef size
        let s' = s + utf8Len t
        writeIORef size s'
        modifyIORef' chunks (t :)
        when (s' > maxOutput) (throwIO Truncated)
      remaining = do
        s <- readIORef size
        pure (max 0 (maxOutput - s))
      emit :: Int -> Val -> IO ()
      emit depth val = do
        n <- atomicModifyIORef' nodes (\k -> (k + 1, k + 1))
        when (n > maxNodes) (throwIO Truncated)
        case val of
          VNil -> append "nil"
          VBool b -> append (if b then "true" else "false")
          VInt i -> append (integerText i)
          VBytes b -> do
            room <- remaining
            let clipped = B.take (room `div` 2 + 1) b
            append (T.pack ("0x" ++ hexLower clipped))
          VText t -> do
            room <- remaining
            let (body, cut) = escapeBounded True room t
            append "\""
            append body
            when cut (append "...")
            append "\""
          VList xs -> container depth "[" "]" xs
          VTuple xs -> container depth "(" ")" xs
      container depth open close xs
        | depth >= maxDepth = append "..."
        | otherwise = do
            append open
            let go _ [] = pure ()
                go first (y : ys) = do
                  when (not first) (append ",")
                  emit (depth + 1) y
                  go False ys
            go True xs
            append close
  r <- tryRecoverable (try (emit 0 v))
  case r of
    Left _ -> pure unrenderable
    Right (Left Truncated) -> do
      cs <- readIORef chunks
      pure (cutBytes maxOutput (T.concat (reverse cs)) <> "...")
    Right (Right ()) -> do
      cs <- readIORef chunks
      pure (T.concat (reverse cs))

-- | Decimal text, or @<bigint N bits>@ above 100,000 bits.
integerText :: Integer -> Text
integerText i
  | abs i >= bigLimit = T.pack ("<bigint " ++ show (integerBitLength (abs i)) ++ " bits>")
  | otherwise = T.pack (show i)

bigLimit :: Integer
bigLimit = 1 `shiftL` maxBigintBits

-- | At most @n@ UTF-8 bytes, cut on a character boundary.
cutBytes :: Int -> Text -> Text
cutBytes n t = T.pack (go 0 (T.unpack t))
  where
    go _ [] = []
    go used (c : cs) =
      let l = utf8Len (T.singleton c)
       in if used + l > n then [] else c : go (used + l) cs

-- | Kind-sensitive equality of two values: @true@ is not @1@, bytes are not
-- text, a list is not a tuple.  Bounded: at most 'maxNodes' nodes compared and
-- nesting at most 64 deep; past either cap the answer is 'False' (fail closed).
-- Pure, but it only forces what it compares, so it is safe on lazy values as
-- long as the cap is reached first.
sameVal :: Val -> Val -> Bool
sameVal a b = maybe False (const True) (go 0 maxNodes a b)
  where
    -- returns the remaining node budget, or Nothing when unequal / over a cap
    go :: Int -> Int -> Val -> Val -> Maybe Int
    go depth budget x y
      | depth > 64 || budget <= 0 = Nothing
      | otherwise = case (x, y) of
          (VNil, VNil) -> Just (budget - 1)
          (VBool p, VBool q) | p == q -> Just (budget - 1)
          (VInt p, VInt q) | p == q -> Just (budget - 1)
          (VBytes p, VBytes q) | p == q -> Just (budget - 1)
          (VText p, VText q) | p == q -> Just (budget - 1)
          (VList p, VList q) -> goList depth (budget - 1) p q
          (VTuple p, VTuple q) -> goList depth (budget - 1) p q
          _ -> Nothing
    goList _ budget [] [] = Just budget
    goList depth budget (p : ps) (q : qs) = go (depth + 1) budget p q >>= \bud -> goList depth bud ps qs
    goList _ _ _ _ = Nothing
