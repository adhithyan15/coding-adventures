-- | The API surface and the edge branches the other suites do not reach:
-- Canonical instances, error text, budget-starved shrinkers (a root with a tiny
-- cap sweeps every refusal branch), decoder caps and TFV float keys.
module ApiSpec (tests) where

import Control.Exception (displayException)
import Control.Monad (forM_)
import qualified Data.ByteString as B
import qualified Data.ByteString.Char8 as BC
import Data.IORef
import qualified Data.Text as T
import Data.Word (Word64, Word8)

import CodingAdventures.TestBootstrap
import CodingAdventures.TestBootstrap.CorpusJson.StrictJson (Json (..))
import CodingAdventures.TestBootstrap.CorpusJson.Values (canonicalInt, decode, showVal)
import qualified CodingAdventures.TestBootstrap.Tfv as Tfv
import CodingAdventures.TestBootstrap.Util
import Harness

off :: Options
off = defaultOptions {optCi = CiOff}

nest :: Int -> Json -> Json
nest 0 j = j
nest n j = nest (n - 1) (JArray [j])

intJ :: B.ByteString -> Json
intJ t = JObject [("kind", JString "int"), ("value", JString t)]

isArg :: PbtError -> Bool
isArg (ArgumentError _) = True
isArg _ = False

isExh :: PbtError -> Bool
isExh (ExhaustedError _) = True
isExh _ = False

decodeFails :: String -> Either String a -> IO ()
decodeFails label = \case
  Left m -> assertTrue (label ++ ": a reason is given") (length m > 0)
  Right _ -> failTest (label ++ ": expected a refusal")

tests :: [Test]
tests =
  [ test "Canonical instances render structurally" $ do
      let r :: Canonical a => a -> IO T.Text
          r = renderText
      a <- r (5 :: Int)
      b <- r (200 :: Word8)
      c <- r (maxBound :: Word64)
      d <- r (B.pack [1, 2])
      e <- r (Just (3 :: Integer))
      f <- r (Nothing :: Maybe Integer)
      g <- r ((1 :: Int), True)
      h <- r ((1 :: Int), True, "t" :: T.Text)
      i <- r ()
      j <- r (VBool True)
      k <- r [Just (1 :: Int), Nothing]
      assertEq "instances" ["5", "200", "18446744073709551615", "0x0102", "(3)", "nil", "(1,true)", "(1,true,\"t\")", "nil", "true", "[(1),nil]"] [a, b, c, d, e, f, g, h, i, j, k]
  , test "error values have text and equality" $ do
      assertEq "arg" "boom" (displayException (ArgumentError "boom"))
      assertEq "not falsifying" "the initial value must falsify the property" (displayException NotFalsifying)
      assertEq "exhausted" "none" (displayException (ExhaustedError "none"))
      assertTrue "eq" (ArgumentError "a" == ArgumentError "a" && ArgumentError "a" /= NotFalsifying)
      assertTrue "show" (length (show (ExhaustedError "x")) > 0)
      assertTrue "ci show" (show CiOn == "CiOn" && CiOff /= CiUnset)
      assertEq "default retries" 100 defaultMaxRetries
  , test "Util: bit length, UTF-8 and hex" $ do
      assertEq "zero" 0 (integerBitLength 0)
      assertEq "negative" 0 (integerBitLength (-5))
      assertEq "one" 1 (integerBitLength 1)
      assertEq "five" 3 (integerBitLength 5)
      assertEq "2^64" 65 (integerBitLength (2 ^ (64 :: Int)))
      assertEq "2^64-1" 64 (integerBitLength (2 ^ (64 :: Int) - 1))
      mapM_
        (\(label, bs) -> assertTrue ("valid " ++ label) (validUtf8 (B.pack bs)))
        [ ("two byte", [0xC3, 0xA9]), ("E0 A0 80", [0xE0, 0xA0, 0x80]), ("ED 9F BF", [0xED, 0x9F, 0xBF])
        , ("EE 80 80", [0xEE, 0x80, 0x80]), ("E1", [0xE1, 0x80, 0x80]), ("F0 90 80 80", [0xF0, 0x90, 0x80, 0x80])
        , ("F1", [0xF1, 0x80, 0x80, 0x80]), ("F4 8F BF BF", [0xF4, 0x8F, 0xBF, 0xBF]), ("empty", []) ]
      mapM_
        (\(label, bs) -> assertFalse ("invalid " ++ label) (validUtf8 (B.pack bs)))
        [ ("E0 9F BF", [0xE0, 0x9F, 0xBF]), ("ED A0 80", [0xED, 0xA0, 0x80]), ("F0 8F BF BF", [0xF0, 0x8F, 0xBF, 0xBF])
        , ("F4 90", [0xF4, 0x90, 0x80, 0x80]), ("C1", [0xC1, 0x80]), ("F5", [0xF5, 0x80, 0x80, 0x80])
        , ("truncated 2", [0xC3]), ("bad cont", [0xC3, 0x41]), ("bad 3 cont", [0xE2, 0x82, 0x41]) ]
      assertEq "hex" "00ff10" (hexLower (B.pack [0, 255, 16]))
      assertTrue "even lowercase" (isLowerHexEven "00ff" && not (isLowerHexEven "00F") && not (isLowerHexEven "0g"))
      assertEq "unhex" (B.pack [0, 255, 16]) (unhex "00ff10")
      assertEq "unhex odd tail ignored" (B.pack [0xab]) (unhex "ab0")
  , test "Values: decoder caps and shapes" $ do
      assertTrue "canonical ints" (all canonicalInt ["0", "7", "-7", "10"] && not (any canonicalInt ["", "-", "-0", "01", "-01", "1a", "+1"]))
      decodeFails "depth 70" (decode (nest 70 (JBool True)))
      assertTrue "depth 30" (either (const False) (const True) (decode (nest 30 (JBool True))))
      decodeFails "oversized list" (decode (JArray (replicate 10001 (JBool True))))
      decodeFails "oversized bytes" (decode (JObject [("kind", JString "bytes"), ("hex", JString (BC.replicate 20002 '0'))]))
      decodeFails "bytes hex not a string" (decode (JObject [("kind", JString "bytes"), ("hex", JNull)]))
      decodeFails "int value not a string" (decode (JObject [("kind", JString "int"), ("value", JNumber "3")]))
      decodeFails "unknown kind" (decode (JObject [("kind", JString "float"), ("value", JString "1")]))
      decodeFails "extra key on an int" (decode (JObject [("kind", JString "int"), ("value", JString "1"), ("x", JNull)]))
      decodeFails "number" (decode (JNumber "1"))
      decodeFails "null" (decode JNull)
      decodeFails "1001 digits" (decode (intJ (BC.replicate 1001 '9')))
      assertTrue "1000 digits" (either (const False) (const True) (decode (intJ (BC.replicate 1000 '9'))))
      decodeFails "1001 digits negative" (decode (intJ (B.cons 0x2d (BC.replicate 1001 '9'))))
      shown <- showVal (VList (replicate 500 (VInt 1234567890)))
      assertTrue "bounded" (T.length shown <= 203)
  , test "TFV: floats, NaN equality and map keys" $ do
      let ok t = assertTrue (BC.unpack t) (Tfv.valid t)
          bad t = assertTrue (BC.unpack t) (not (Tfv.valid t))
      ok "tfv 1\nr R1 Kf D3ff0000000000000\n"
      ok "tfv 1\nr R1 Kf D7ff8000000000000\n"
      ok "tfv 1\nr R1 Kf D7ff0000000000000\n"
      bad "tfv 1\nr R1 Km M2 D7ff8000000000000 I1 D7ff8000000000001 I2\n" -- every NaN equals every NaN
      ok "tfv 1\nr R1 Km M2 D0000000000000000 I1 D8000000000000000 I2\n" -- -0 and +0 stay distinct
      bad "tfv 1\nr R1 Km M2 D3ff0000000000000 I1 D3ff0000000000000 I2\n"
      ok "tfv 1\nr R1 Km M2 L1 I1 I1 L1 I2 I2\n"
      bad "tfv 1\nr R1 Km M2 L1 I1 I1 L1 I1 I2\n"
      bad "tfv 1\nr R1 Kf D3FF0000000000000\n"
      ok "tfv 1\nr R1 Kt S\n"
      ok "tfv 1\nr R1 Kt S61c3a9\n"
      ok "tfv 1\nr R2 Ka X Kb S\n"
      bad "tfv 1\nr R1 Kl L1\n"
      bad "tfv 1\nr R1 Km M1 I1\n"
  , test "shrinkers under a root with a tiny cap hit every refusal branch and stay sound" $
      forM_ [0 .. 70 :: Int] $ \cap -> do
        let prop l = pure (sum l <= 20)
            list1 = [10, 5, 8, 4, 9, 1]
        (m, _) <- withRoot defaultConfig cap (\root -> shrinkListWith defaultConfig (Just root) list1 0 1000 prop)
        assertTrue ("list sound at cap " ++ show cap) (sum m > 20)
        (n, _) <- withRoot defaultConfig cap (\root -> shrinkIntWith defaultConfig (Just root) 54321 0 0 100000 1000 (pure . (<= 100)))
        assertTrue ("int sound at cap " ++ show cap) (n > 100)
        (bs, _) <- withRoot defaultConfig cap (\root -> shrinkBytesWith defaultConfig (Just root) (B.pack [1 .. 9]) 1000 (pure . (\b -> B.length b < 5)))
        assertTrue ("bytes sound at cap " ++ show cap) (B.length bs >= 5)
        (rt, _) <- withRoot defaultConfig cap (\root -> shrinkGen defaultConfig (Just root) (bytes 0 20) (B.pack [1 .. 12]) (pure . (\b -> B.length b >= 5)) 1000)
        assertTrue ("gen bytes sound at cap " ++ show cap) (B.length rt >= 5)
        (ls, _) <- withRoot defaultConfig cap (\root -> shrinkGen defaultConfig (Just root) (list (int 0 50) 0 10) [5, 6, 7, 8, 9, 10, 11] (pure . ((> 20) . sum)) 1000)
        assertTrue ("gen list sound at cap " ++ show cap) (sum ls > 20)
  , test "simulateReplay and the shrinkers are starved by a spent root without running the property" $ do
      calls <- newIORef (0 :: Int)
      let prop _ = modifyIORef' calls (+ 1) >> pure False
      r <- withRoot defaultConfig 0 (\root -> (,) <$> shrinkListWith defaultConfig (Just root) [1, 2] 0 1000 prop <*> shrinkBytesWith defaultConfig (Just root) "ab" 1000 prop)
      assertEq "starved" (([1, 2], 0), ("ab", 0)) r
      assertThrows "replay needs draws" isArg (withRoot defaultConfig 0 (\root -> simulateReplayWith defaultConfig (Just root) 1 prop))
      n <- readIORef calls
      assertEq "property never ran" 0 n
  , test "generator edge branches" $ do
      rng <- newRng 3
      -- tuple2 sampler and shrinker, ordered draws
      pair <- sample Nothing (tuple2 (int 0 9) bool) rng
      assertTrue "pair" (fst pair >= 0)
      ((a, b), _) <- shrinkGen defaultConfig Nothing (tuple2 (int 0 100) (int 0 100)) (50, 60) (pure . (\(x, y) -> x + y >= 30)) 1000
      assertEq "sum" 30 (a + b)
      -- a tuple whose budget runs out between coordinates
      (t1, s1) <- shrinkGen defaultConfig Nothing (tuple2 (int 0 100) (int 0 100)) (50, 60) (pure . const True) 1
      assertEq "one step only" 1 s1
      assertTrue "first coordinate moved" (fst t1 == 0)
      (t3, s3) <- shrinkGen defaultConfig Nothing (tuple3 (int 0 100) (int 0 100) (int 0 100)) (50, 60, 70) (pure . const True) 2
      assertEq "two steps" 2 s3
      assertEq "triple" (0, 0, 70) t3
      (t4, _) <- shrinkGen defaultConfig Nothing (tuple3 (int 0 100) (int 0 100) (int 0 100)) (50, 60, 70) (pure . (\(x, y, z) -> x + y + z >= 90)) 1000
      assertTrue "triple still falsifies" (let (x, y, z) = t4 in x + y + z >= 90)
      -- the shrinkerless generators return their value untouched
      forM_ [("map", shrinkGen defaultConfig Nothing (mapG (+ 1) (int 0 9)) 5 (pure . const True) 10), ("flat_map", shrinkGen defaultConfig Nothing (flatMap constant (int 0 9)) 5 (pure . const True) 10), ("one_of", shrinkGen defaultConfig Nothing (oneOf [int 0 9]) 5 (pure . const True) 10), ("frequency", shrinkGen defaultConfig Nothing (frequency [(1, int 0 9)]) 5 (pure . const True) 10), ("constant", shrinkGen defaultConfig Nothing (constant 5) 5 (pure . const True) 10)] $ \(label, act) -> do
        r <- act
        assertEq label (5 :: Integer, 0) r
      -- frequency walks past earlier arms
      vs <- mapM (const (sample Nothing (frequency [(1, constant 'a'), (1, constant 'b'), (1, constant 'c')]) rng)) [1 .. 60 :: Int]
      assertTrue "all three arms" (all (`elem` vs) ['a', 'b', 'c'])
      -- bytes shrinking with no budget and with a spent budget
      z <- shrinkGen defaultConfig Nothing (bytes 0 10) (B.pack [1, 2, 3]) (pure . const True) 0
      assertEq "no budget" (B.pack [1, 2, 3], 0) z
      -- sampling a filter under a scope charges attempts when nested, work when top-level
      withRoot defaultConfig 100 $ \root -> do
        v <- sample (Just root) (filterG 5 (const True) (int 0 3)) rng
        assertTrue "sampled" (v >= 0)
        w <- work root
        assertEq "top-level scope charges work" 1 w
      withRoot defaultConfig 100 $ \root -> withChild defaultConfig (Just root) 10 $ \child -> do
        _ <- sample (Just child) (filterG 5 (const True) (int 0 3)) rng
        a1 <- attempts child
        assertEq "nested scope charges an attempt" 1 a1
      -- a spent scope makes the filter exhausted
      assertThrows "spent filter" isExh (withRoot defaultConfig 0 (\root -> sample (Just root) (filterG 5 (const True) (int 0 3)) rng))
      -- the u64 and i128 shrinkers
      (u, _) <- shrinkGen defaultConfig Nothing u64 12345 (pure . const True) 10
      assertEq "u64" 0 u
      (i, _) <- shrinkGen defaultConfig Nothing i128 (-12345) (pure . const True) 10
      assertEq "i128" 0 i
      (c, _) <- shrinkGen defaultConfig Nothing u8 200 (pure . const True) 10
      assertEq "u8" 0 c
  , test "genPrimitive, genBounded and genBytes cover every branch" $ do
      b <- genPrimitive defaultConfig 0 "bool" 3
      u <- genPrimitive defaultConfig 0 "u8" 2
      w <- genPrimitive defaultConfig 0 "u64" 2
      assertEq "lengths" [3, 2, 2] [length b, length u, length w]
      assertThrows "wide span x count" (\e -> case e of ArgumentError m -> "budget" `elem` words m; _ -> False) (genBounded defaultConfig 1 0 (2 ^ (4000 :: Int)) 1000000)
  , test "runner sizes: nested checks charge collections of every shape" $ do
      r1 <- withRoot defaultConfig 100 (\root -> checkWith (Just root) Nothing 1 (customGen (\_ _ -> pure (list0, 3 :: Integer)) Nothing) off {optNumTests = 2} (\_ -> pure True))
      assertTrue "tuple passes under a roomy root" (vPassed r1)
      r2 <- withRoot defaultConfig 100 (\root -> checkWith (Just root) Nothing 1 (customGen (\_ _ -> pure (list0, list0)) Nothing) off {optNumTests = 2} (\_ -> pure True))
      assertTrue "pair of lists passes under a roomy root" (vPassed r2)
  , test "nested checks over bytes, texts and tuples are charged as work" $ do
      let texts = customGen (\_ _ -> pure (T.replicate 50 "é")) Nothing
          bss = customGen (\_ _ -> pure (B.replicate 50 1)) Nothing
          tups = customGen (\_ _ -> pure [(1 :: Integer, "ab" :: T.Text), (2, "cd")]) Nothing
      withRoot defaultConfig 100 $ \root -> do
        _ <- checkWith (Just root) Nothing 1 texts off {optNumTests = 1} (\_ -> pure True)
        w1 <- work root
        assertTrue "text work" (w1 >= 100)
        _ <- checkWith (Just root) Nothing 1 bss off {optNumTests = 1} (\_ -> pure True)
        _ <- checkWith (Just root) Nothing 1 tups off {optNumTests = 1} (\_ -> pure True)
        w2 <- work root
        assertTrue "bytes and tuples add work" (w2 > w1 + 50)
  , test "valWork is bounded and total" $ do
      a <- valWork (VList (replicate 20000 (VList [VInt 1, VInt 2])))
      assertTrue "bounded by the visit cap" (a <= 10000 + 10000 * 2)
      b <- valWork (VTuple [VText "ab", VBytes "cd", VList [VNil]])
      assertTrue "shallow children counted" (b >= 3)
      c <- valWork (VList [VInt 1, error "hostile"])
      assertEq "a hostile value counts 0" 0 c
      d <- valWork (VBool True)
      assertEq "scalar" 0 d
  ]
  where
    list0 = [1 .. 10 :: Integer]
