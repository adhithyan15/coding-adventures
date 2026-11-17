-- | Bit-exact conformance of "Trig" against the shared TRIG-REF-1 vectors.
--
-- @code/specs/fixtures/trig-reference-v1@ holds every vector as a tab-separated row
--
-- > op <TAB> x <TAB> y <TAB> cr <TAB> flags
--
-- where @x@ and @y@ are IEEE binary64 /bit patterns/ (16 lowercase hex digits),
-- never decimals.  This module decodes @x@ by a bit cast ('castWord64ToDouble'),
-- calls the public function, encodes the result back to bits ('castDoubleToWord64')
-- and compares the two __integers__ (PHY00a 6.5).  Printed floats are never
-- compared: they hide the last digits.  The one relaxation is NaN: a reference
-- result of @7ff8000000000000@ matches any NaN (VER04p @same@).
--
-- The test is run from the package directory (that is where @cabal test@ runs a
-- test suite), so the fixture lives at @../../../specs/fixtures/trig-reference-v1@.
module ConformanceSpec (spec) where

import Data.Bits (shiftL, (.|.))
import Data.Char (digitToInt, isAsciiLower, isDigit)
import Data.Either (isLeft)
import Data.List (group, isPrefixOf, sort, sortOn, tails)
import Data.Word (Word64)
import GHC.Float (castDoubleToWord64, castWord64ToDouble)
import Test.Hspec
import Prelude hiding (atan, atan2, cos, sin, sqrt, tan)
import qualified Trig
import qualified Trig.Constants as K

fixtureDir :: FilePath
fixtureDir = "../../../specs/fixtures/trig-reference-v1"

-- | The four groups and their row counts, as recorded in @manifest.json@.
groups :: [(String, Int)]
groups =
    [ ("edge", 2854)
    , ("sweep-binade", 19520)
    , ("sweep-cw", 19300)
    , ("sweep-prng", 20480)
    ]

-- | One vector: the operation, the argument field, the reference result field,
-- the correctly rounded field and the flags field, all as written in the file.
data Row = Row
    { rowOp :: String
    , rowX :: String
    , rowY :: String
    , rowCr :: String
    , rowFlags :: String
    }

splitTabs :: String -> [String]
splitTabs line = case break (== '\t') line of
    (field, []) -> [field]
    (field, _ : rest) -> field : splitTabs rest

parseRow :: String -> Row
parseRow line = case splitTabs line of
    [op, x, y, cr, flags] -> Row op x y cr flags
    fields -> error ("malformed vector row (" ++ show (length fields) ++ " fields): " ++ line)

-- | Sixteen lowercase hex digits to a 'Word64'.  Anything else is a corpus error.
hexBits :: String -> Word64
hexBits field
    | length field == 16 && all isHex field = foldl (\acc c -> (acc `shiftL` 4) .|. fromIntegral (digitToInt c)) 0 field
    | otherwise = error ("not 16 lowercase hex digits: " ++ show field)
  where
    isHex c = isDigit c || (isAsciiLower c && c <= 'f')

toDouble :: String -> Double
toDouble = castWord64ToDouble . hexBits

bitsOf :: Double -> Word64
bitsOf = castDoubleToWord64

qnanBits :: Word64
qnanBits = 0x7FF8000000000000

loadGroup :: String -> IO [Row]
loadGroup name = do
    text <- readFile (fixtureDir ++ "/" ++ name ++ ".tsv")
    pure (map parseRow (lines text))

-- | Call the port's public function for one row.  'Nothing' stands for the
-- port's own error (Haskell's @sqrt@ returns @Left@ for a negative argument).
call :: String -> String -> Maybe Double
call op x = case op of
    "sin" -> Just (Trig.sin arg)
    "cos" -> Just (Trig.cos arg)
    "tan" -> Just (Trig.tan arg)
    "radians" -> Just (Trig.radians arg)
    "degrees" -> Just (Trig.degrees arg)
    "atan" -> Just (Trig.atan arg)
    "sqrt" -> either (const Nothing) Just (Trig.sqrt arg)
    "atan2" ->
        let (ys, xs) = break (== ',') x
         in Just (Trig.atan2 (toDouble ys) (toDouble (drop 1 xs)))
    _ -> error ("unknown op in the corpus: " ++ op)
  where
    arg = toDouble x

-- | Why a row failed, or 'Nothing' if the port matches the reference bit for bit.
check :: Row -> Maybe String
check row
    | rowY row == "error" = case call (rowOp row) (rowX row) of
        Nothing -> Nothing
        Just v -> Just ("expected an error, got " ++ hex v)
    | otherwise = case call (rowOp row) (rowX row) of
        Nothing -> Just "unexpected error"
        Just got
            | want == qnanBits -> if isNaN got then Nothing else Just ("expected NaN, got " ++ hex got)
            | bitsOf got == want -> Nothing
            | otherwise -> Just ("expected " ++ rowY row ++ ", got " ++ hex got)
          where
            want = hexBits (rowY row)
  where
    hex v = showHex16 (bitsOf v)

showHex16 :: Word64 -> String
showHex16 w = [digits !! fromIntegral ((w `div` (16 ^ i)) `mod` 16) | i <- [15, 14 .. 0 :: Int]]
  where
    digits = "0123456789abcdef"


-- | Position of a double on the number line as a signed integer: adjacent doubles
-- differ by one, and @-0@ and @+0@ coincide (the usual ulp-distance ordering).
ordered :: Word64 -> Integer
ordered bits
    | bits >= 0x8000000000000000 = negate magnitude
    | otherwise = magnitude
  where
    magnitude = fromIntegral (bits `mod` 0x8000000000000000)

-- | Accuracy limit, in steps between adjacent doubles, against the correctly
-- rounded column.  @tan@ is named as a no-claim op: PHY00a 3.8 makes no accuracy
-- claim for it (it is @sin / cos@ plus a division and a pinned pole sentinel).
accuracyLimit :: String -> Maybe (Maybe Integer)
accuracyLimit op = case op of
    "sin" -> Just (Just 3)
    "cos" -> Just (Just 3)
    "radians" -> Just (Just 1)
    "degrees" -> Just (Just 1)
    "tan" -> Just Nothing
    _ -> Nothing

-- | Summarise failures as "op: count" lines plus the first few rows.
summary :: [(Row, String)] -> String
summary failures =
    unlines $
        [ op ++ ": " ++ show (length ops) ++ " failing rows"
        | ops@(op : _) <- group (sort (map (rowOp . fst) failures))
        ]
            ++ [ "  " ++ rowOp r ++ " " ++ rowX r ++ ": " ++ why
               | (r, why) <- take 10 failures
               ]

-- | The @constants@ object of @manifest.json@ as (name, bits) pairs.  The object is
-- flat (string keys, 16-digit hex string values), so a split on double quotes is
-- enough: the pieces alternate between punctuation and strings.
manifestConstants :: IO [(String, Word64)]
manifestConstants = do
    text <- readFile (fixtureDir ++ "/manifest.json")
    let afterKey = drop (length "\"constants\"") (head ([t | t <- tails text, "\"constants\"" `isPrefixOf` t] ++ [error "no constants in manifest"]))
        body = takeWhile (/= '}') afterKey
        strings = [piece | (i, piece) <- zip [0 :: Int ..] (splitQuotes body), odd i]
        pairUp (k : v : rest) = (k, hexBits v) : pairUp rest
        pairUp _ = []
    pure (pairUp strings)
  where
    splitQuotes str = case break (== '"') str of
        (piece, []) -> [piece]
        (piece, _ : rest) -> piece : splitQuotes rest

spec :: Spec
spec = do
    describe "constants are the pinned bit patterns" $ do
        it "every manifest constant is in the module, bit for bit, and nothing else is" $ do
            manifest <- manifestConstants
            let ours = [(name, bitsOf v) | (name, v) <- K.constantTable]
            sort ours `shouldBe` sort manifest
        it "the exported circle constants are the manifest's" $ do
            map bitsOf [Trig.piValue, Trig.twoPi, Trig.halfPi]
                `shouldBe` [0x400921FB54442D18, 0x401921FB54442D18, 0x3FF921FB54442D18]
        it "the degree constants equal the divisions of PI" $ do
            K.radPerDeg `shouldBe` Trig.piValue / 180.0
            K.degPerRad `shouldBe` 180.0 / Trig.piValue
        it "decimal literals agree with the pinned legacy bits" $ do
            bitsOf 1.0e-15 `shouldBe` bitsOf K.tanPole
            bitsOf 1.0e308 `shouldBe` bitsOf K.tanBig
            bitsOf 1.0e-17 `shouldBe` bitsOf K.atanTerm
            bitsOf 1.0e-300 `shouldBe` bitsOf K.sqrtTiny
            bitsOf 7.450580596923828e-9 `shouldBe` bitsOf K.tiny

    describe "TRIG-REF-1 vectors, bit for bit" $
        mapM_
            ( \(name, count) -> it ("every row of " ++ name ++ " matches") $ do
                rows <- loadGroup name
                length rows `shouldBe` count
                let failures = [(r, why) | r <- rows, Just why <- [check r]]
                if null failures
                    then pure ()
                    else expectationFailure (name ++ ": " ++ show (length failures) ++ " of " ++ show count ++ " rows differ\n" ++ summary failures)
            )
            groups

    describe "accuracy against the correctly rounded column" $
        it "sin and cos within 3 steps, radians and degrees within 1, tan has no claim" $ do
            rows <- concat <$> mapM (loadGroup . fst) groups
            let withCr = [r | r <- rows, rowCr r /= "-"]
                outcomes =
                    [ (rowOp r, steps)
                    | r <- withCr
                    , Just (Just _) <- [accuracyLimit (rowOp r)]
                    , Just got <- [call (rowOp r) (rowX r)]
                    , let steps = abs (ordered (bitsOf got) - ordered (hexBits (rowCr r)))
                    ]
                -- Fail closed: an op with a cr value but no stated limit (and not
                -- named as no-claim) is a failure, so a new op cannot be skipped
                -- silently.  An unknown op fails the same way.
                unlimited = sort [op | r <- withCr, let op = rowOp r, accuracyLimit op == Nothing]
            unlimited `shouldBe` []
            let worst = [(op, maximum (map snd grp)) | grp@((op, _) : _) <- groupByOp outcomes]
            mapM_
                ( \(op, w) -> case accuracyLimit op of
                    Just (Just bound) -> (op, w <= bound) `shouldBe` (op, True)
                    _ -> pure ()
                )
                worst
            -- the gate really measured something for each claimed op
            map fst worst `shouldBe` ["cos", "degrees", "radians", "sin"]

    describe "contract (PHY00a 3.3 and 3.8)" $ do
        it "sin(-0) is -0 and sin(+0) is +0" $ do
            bitsOf (Trig.sin (-0.0)) `shouldBe` 0x8000000000000000
            bitsOf (Trig.sin 0.0) `shouldBe` 0
        it "cos(+-0) is 1" $ do
            Trig.cos 0.0 `shouldBe` 1.0
            Trig.cos (-0.0) `shouldBe` 1.0
        it "tiny arguments are returned unchanged" $ do
            let tiny = castWord64ToDouble 0x3E3FFFFFFFFFFFFF
            Trig.sin tiny `shouldBe` tiny
            Trig.sin 5.0e-324 `shouldBe` 5.0e-324
            Trig.cos tiny `shouldBe` 1.0
        it "sin, cos and tan are NaN outside |x| <= 2^29" $
            mapM_
                ( \x -> do
                    Trig.sin x `shouldSatisfy` isNaN
                    Trig.cos x `shouldSatisfy` isNaN
                    Trig.tan x `shouldSatisfy` isNaN
                )
                [ 0 / 0
                , 1 / 0
                , -1 / 0
                , 1.0e20
                , 2 ^^ (29 :: Int) * (1 + 2 ^^ (-52 :: Int))
                , 1.7976931348623157e308
                ]
        it "the last in-domain value is finite" $ do
            bitsOf (Trig.sin (2 ^^ (29 :: Int))) `shouldBe` 0x3FD4E67C0E2622DF
            bitsOf (Trig.cos (2 ^^ (29 :: Int))) `shouldBe` 0xBFEE3EDD2DFEEF90
        it "radians and degrees are single multiplies" $ do
            Trig.radians 180.0 `shouldBe` Trig.piValue
            Trig.degrees Trig.piValue `shouldBe` 180.0
            Trig.degrees 1.7e308 `shouldBe` (1 / 0)
            Trig.radians (0 / 0) `shouldSatisfy` isNaN
            bitsOf (Trig.radians (-0.0)) `shouldBe` 0x8000000000000000
            bitsOf (Trig.degrees (-0.0)) `shouldBe` 0x8000000000000000
        it "tan keeps the +-1e308 pole sentinel" $ do
            Trig.tan Trig.halfPi `shouldBe` 1.0e308
            Trig.tan (negate Trig.halfPi) `shouldBe` (-1.0e308)
        it "atan2 of a NaN argument is NaN" $ do
            let nan = 0 / 0 :: Double
            mapM_ (\(y, x) -> Trig.atan2 y x `shouldSatisfy` isNaN)
                [(nan, 1.0), (1.0, nan), (-1.0, nan), (nan, nan), (nan, -1.0), (0.0, nan)]
        it "atan2 keeps its pinned conventions" $ do
            Trig.atan2 (-0.0) (-1.0) `shouldBe` Trig.piValue
            Trig.atan2 0.0 0.0 `shouldBe` 0.0
            Trig.atan2 (1 / 0) (1 / 0) `shouldSatisfy` isNaN
            Trig.atan2 1.0e300 1.0e-300 `shouldBe` Trig.halfPi
        it "atan keeps the sign of zero" $
            bitsOf (Trig.atan (-0.0)) `shouldBe` 0x8000000000000000
        it "sqrt of a negative is Left" $
            Trig.sqrt (-1.0) `shouldSatisfy` isLeft
        it "negative quadrants use the floor modulus" $ do
            -- k = -1 and k = -5 both land in quadrant 3.
            Trig.sin (negate Trig.halfPi) `shouldBe` negate (Trig.sin Trig.halfPi)
            abs (Trig.cos (-5 * Trig.halfPi)) `shouldSatisfy` (< 1.0e-15)

groupByOp :: [(String, Integer)] -> [[(String, Integer)]]
groupByOp = foldr insert [] . sortOn fst
  where
    insert x [] = [[x]]
    insert x (g@(y : _) : gs)
        | fst x == fst y = (x : g) : gs
        | otherwise = [x] : g : gs
    insert _ ([] : _) = error "unreachable"
