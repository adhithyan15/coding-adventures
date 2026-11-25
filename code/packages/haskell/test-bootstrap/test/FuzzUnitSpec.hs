-- | Pure fuzz-engine units and the PINNED values the fuzz-v1 corpus cannot catch
-- (VER06o 9): every interest value and delta, unequal splice splits, the power
-- schedule (exact half-way ties), real sig_hash values, the rejection draw in
-- bounded sampling, the minimiser's bisection halves and the TAP token bytes.
--
-- Cross-lane values were produced by running the Ruby lane (VER06i) and the
-- Python oracle on the same inputs.
module FuzzUnitSpec (tests) where

import Control.Exception (SomeException, try)
import Control.Monad (forM_)
import qualified Data.ByteString as B
import Data.IORef
import Data.Text (Text)
import qualified Data.Text as T
import Data.Word (Word64)
import System.CPUTime (getCPUTime)

import CodingAdventures.TestBootstrap.Error (PbtError (..))
import CodingAdventures.TestBootstrap.Fuzz.Bitmap
import CodingAdventures.TestBootstrap.Fuzz.Engine
import CodingAdventures.TestBootstrap.Fuzz.FailureIds
import CodingAdventures.TestBootstrap.Fuzz.Limits
import CodingAdventures.TestBootstrap.Fuzz.Mutate
import CodingAdventures.TestBootstrap.Fuzz.Signature
import CodingAdventures.TestBootstrap.Fuzz.Types
import CodingAdventures.TestBootstrap.Prng
import CodingAdventures.TestBootstrap.Util (hexLower, unhex)
import qualified Data.ByteString.Char8 as BC
import Harness
import Vectors

hx :: String -> B.ByteString
hx = unhex . BC.pack

hex :: B.ByteString -> String
hex = hexLower

isArg :: PbtError -> Bool
isArg (ArgumentError _) = True
isArg _ = False

-- | The seed whose FIRST SplitMix64 word is the given one (the finalizer inverted),
-- computed with an independent Python script and checked here against 'nextU64'.
seedFor :: Integer -> Word64
seedFor w = case lookup w table of
  Just s -> s
  Nothing -> error "seedFor: unpinned word"
  where
    table =
      [ (0, 7046029254386353131)
      , (1, 17885559969949501885)
      , (2, 10278346628982968224)
      , (3, 9697084067704644217)
      , (2 ^ (63 :: Int) - 2, 16898060955550185018)
      , (2 ^ (63 :: Int) - 1, 959135552437182909)
      ]

energyRows :: [(Int, Int, Int, Int, Int)]
energyRows =
  [ (10, 4, 3, 6, 8), (10, 1, 0, 0, 10), (10, 4, 0, 0, 5), (1, 4, 0, 0, 1), (10, 16, 0, 0, 3)
  , (10, 100, 0, 0, 1), (3, 4, 0, 0, 2), (5, 4, 0, 0, 3), (10, 9, 0, 0, 3), (10, 2, 1, 2, 11)
  , (7, 1, 1, 3, 9), (0, 4, 0, 0, 1), (10, 4, 1, 1, 10), (25, 100, 0, 0, 3), (10, 25, 0, 0, 2)
  , (15, 1, 0, 0, 15), (10, 1, 5, 5, 20)
  ]

sigs :: [(Text, [Text], Text)]
sigs =
  [ ("panic", ["eval", "step"], "4eca7ae8")
  , ("segv", ["alloc", "write"], "a0aa3aa2")
  , ("timeout", [], "4e61ea76")
  , ("oom", [], "7146a410")
  , ("RuntimeError", ["t.rb#Foo.bar", "t.rb#block_in_Foo.bar"], "c12b6a11")
  , ("stale-budget-sentinel", [], "8a20b3bf")
  , ("ArgumentError", ["a.rb#x"], "c93b2322")
  ]

-- | A probe that records every candidate it is shown.
traceProbe :: (B.ByteString -> Bool) -> IO (B.ByteString -> IO Outcome, IO [String])
traceProbe predicate = do
  seen <- newIORef []
  let probe c = do
        modifyIORef' seen (hex c :)
        pure (if predicate c then Crash "K" ["f"] else Pass)
  pure (probe, reverse <$> readIORef seen)

sigK :: Text
sigK = crashSignature "K" ["f"]

tests :: [Test]
tests =
  [ test "every interest value and delta is pinned" $ do
      assertEq "i8" [0, 1, 127, 128, 255] interest8
      assertEq "i16" [0, 1, 255, 256, 32767, 32768, 65535] interest16
      assertEq "i32" [0, 1, 65535, 65536, 0x7FFFFFFF, 0x80000000, 0xFFFFFFFF] interest32
      assertEq "i64" [0, 1, 0xFFFFFFFF, 0x100000000, 0x7FFFFFFFFFFFFFFF, 0x8000000000000000, 0xFFFFFFFFFFFFFFFF] interest64
      assertEq "deltas" [-35, -8, -1, 1, 8, 35] deltas
      assertEq "by width" [1, 2, 4, 8] (map fst interestByWidth)
  , test "every interest value round-trips through boundaryInsert in both endiannesses" $
      forM_ interestByWidth $ \(w, vals) -> forM_ vals $ \v -> do
        let zero = B.replicate (w + 2) 0
            le = boundaryInsert zero 1 (toInteger w) v Little
            be = boundaryInsert zero 1 (toInteger w) v Big
            rd bs = B.foldl' (\a b -> a * 256 + toInteger b) 0 bs
        assertEq "le" v (rd (B.reverse (B.take w (B.drop 1 le))))
        assertEq "be" v (rd (B.take w (B.drop 1 be)))
        assertEq "edges" (B.pack [0, 0]) (B.pack [B.head le, B.last le])
  , test "arithmetic wraps modulo the width in every width and endianness" $ do
      forM_ [1, 2, 4, 8] $ \w -> forM_ [Little, Big] $ \e -> do
        let ff = B.replicate (fromInteger w) 0xFF
            zero = B.replicate (fromInteger w) 0
        assertEq "ff+1" (replicate (2 * fromInteger w) '0') (hex (byteArithmetic ff 0 1 w e))
        assertEq "0-1" (replicate (2 * fromInteger w) 'f') (hex (byteArithmetic zero 0 (-1) w e))
        assertEq "2^(8w)" ff (byteArithmetic ff 0 (2 ^ (8 * w)) w e)
      assertEq "u16 le" "0001" (hex (byteArithmetic (hx "ff00") 0 1 2 Little))
      assertEq "u16 be" "ff01" (hex (byteArithmetic (hx "ff00") 0 1 2 Big))
  , test "unsupported widths and short buffers return the input unchanged" $ do
      forM_ [0, 3, 5, 16, -1, 2 ^ (70 :: Int)] $ \w -> do
        assertEq "arith" "0102" (hex (byteArithmetic (hx "0102") 0 1 w Little))
        assertEq "bound" "0102" (hex (boundaryInsert (hx "0102") 0 w 9 Little))
      assertEq "short" "01" (hex (byteArithmetic (hx "01") 0 1 2 Little))
      assertEq "empty" "" (hex (boundaryInsert B.empty 0 1 9 Little))
  , test "unequal splice splits with target and donor swapped" $ do
      let a = hx "01020304"
          b = hx "0506070809"
      assertEq "1,3" "010809" (hex (splice a b 1 3))
      assertEq "swapped 3,1" "050607020304" (hex (splice b a 3 1))
      assertEq "4,4" "0102030409" (hex (splice a b 4 4))
      assertEq "5,2" "05060708090304" (hex (splice b a 5 2))
      assertEq "2,2" "0102070809" (hex (splice a b 2 2))
      assertTrue "differ" (splice a b 1 3 /= splice b a 3 1)
  , test "splice clamps both splits" $ do
      let a = hx "0102"
          b = hx "0304"
      assertEq "high" "0102" (hex (splice a b 99 99))
      assertEq "low" "0304" (hex (splice a b (-5) (-5)))
      assertEq "huge" "01020304" (hex (splice a b (2 ^ (80 :: Int)) (negate (2 ^ (80 :: Int)))))
      assertEq "empty" "" (hex (splice B.empty B.empty 1 1))
  , test "flips wrap and use parity for any count" $ do
      let z = hx "0000"
      assertEq "9" "0002" (hex (bitFlip z 9 1))
      assertEq "-7 (floored)" "0002" (hex (bitFlip z (-7) 1))
      assertEq "1,15" "feff" (hex (bitFlip z 1 15))
      assertEq "15,15" "ffbf" (hex (bitFlip z 15 15))
      assertEq "0,17" "feff" (hex (bitFlip z 0 17))
      assertEq "5,32" "0000" (hex (bitFlip z 5 32))
      assertEq "5,16" "ffff" (hex (bitFlip z 5 16))
      assertEq "15,1" "0080" (hex (bitFlip z 15 1))
      assertEq "byte" "ed3456" (hex (byteFlip (hx "123456") 0 1))
      assertEq "byte 1,2" "12cba9" (hex (byteFlip (hx "123456") 1 2))
      assertEq "byte -1,2" "ed34a9" (hex (byteFlip (hx "123456") (-1) 2))
      assertEq "byte 0,3" "edcba9" (hex (byteFlip (hx "123456") 0 3))
      assertEq "byte 1,6" "123456" (hex (byteFlip (hx "123456") 1 6))
      assertEq "empty" "" (hex (bitFlip B.empty 3 3))
      assertEq "count 0" "0102" (hex (byteFlip (hx "0102") 0 0))
      assertEq "count -4" "0102" (hex (byteFlip (hx "0102") 0 (-4)))
  , test "a 1 MiB flip with a 1,000-digit count is cheap and builds no big Integer" $ do
      let big = B.replicate maxInputBytes 0
          count' = 10 ^ (1000 :: Int) + 7 :: Integer
      t0 <- getCPUTime
      let out = bitFlip big 12345 count'
          out2 = byteFlip big 12345 count'
      assertEq "len" maxInputBytes (B.length out)
      assertEq "len2" maxInputBytes (B.length out2)
      -- force both
      assertTrue "changed" (B.any (/= 0) out && B.any (/= 0) out2)
      t1 <- getCPUTime
      assertTrue "under 5 s of CPU" ((t1 - t0) < 5 * 10 ^ (12 :: Int))
      assertEq "single byte flip" 255 (B.index (byteFlip big 3 1) 3)
  , test "flips are involutions" $ do
      let d = hx "0a1b2c3d4e"
      forM_ [0, 1, 9, 39, 40, 2 ^ (65 :: Int)] $ \off -> forM_ [1, 4, 17, 2 ^ (64 :: Int) + 5] $ \c -> do
        assertEq "bit" d (bitFlip (bitFlip d off c) off c)
        assertEq "byte" d (byteFlip (byteFlip d off c) off c)
  , test "block operations and their clamping" $ do
      assertEq "del" "010405" (hex (blockDelete (hx "0102030405") 1 2))
      assertEq "del clamp" "0102" (hex (blockDelete (hx "0102030405") 2 99))
      assertEq "del 0" "0102030405" (hex (blockDelete (hx "0102030405") 2 0))
      assertEq "del empty" "" (hex (blockDelete B.empty 2 2))
      assertEq "ins empty" "0304" (hex (blockInsert B.empty 5 (hx "0304")))
      assertEq "ins mid" "0102030405" (hex (blockInsert (hx "010205") 2 (hx "0304")))
      assertEq "ins end" "010203" (hex (blockInsert (hx "0102") 2 (hx "03")))
      assertEq "dup" "0102030102" (hex (blockDuplicate (hx "010203") 0 2 3))
      assertEq "dup 0" "010203" (hex (blockDuplicate (hx "010203") 0 0 3))
      assertEq "dup empty" "" (hex (blockDuplicate B.empty 0 1 0))
      assertEq "dup clamp" "010303" (hex (blockDuplicate (hx "0103") 1 9 2))
  , test "sequence edges, caps and the length budget" $ do
      s0 <- mutateSequence (hx "0102") 1 0 100
      assertEq "0 steps" "0102" (hex s0)
      s1 <- mutateSequence (hx "0102") 1 (-5) 100
      assertEq "negative steps" "0102" (hex s1)
      e <- mutateSequence B.empty 7 1 100
      assertTrue "refilled" (not (B.null e))
      assertThrows "steps over cap" isArg (mutateSequence (hx "61") 1 (toInteger maxIterations + 1) 100)
      assertThrows "input over cap" isArg (mutateSequence (B.replicate (maxInputBytes + 1) 97) 1 1 100)
      forM_ [0 .. 199] $ \seed -> do
        o <- mutateSequence (hx "0001020304050607") seed 40 8
        assertTrue "max_len" (B.length o <= 8)
      grown <- mutateSequence (hx "0001020304050607") 3 40 maxInputBytes
      assertTrue "grows" (B.length grown > 8)
  , test "sequence work is the bytes actually copied, failed calls included" $ do
      r0 <- mutateSequenceCounted (hx "0102") 5 0 100 100
      assertEq "initial copy only" (WorkOk (hx "0102") 2) r0
      -- 100 initial + 100 for the first step; the second step would pass 250
      r1 <- mutateSequenceCounted (B.replicate 100 97) 1 10 maxInputBytes 250
      assertEq "stops at the step that would cross" (WorkExhausted 200) r1
      r2 <- mutateSequenceCounted (B.replicate 100 97) 1 10 maxInputBytes 99
      assertEq "initial copy alone is over" (WorkExhausted 0) r2
      r3 <- mutateSequenceCounted (B.replicate maxInputBytes 97) 1 100 (2 * maxInputBytes) (4 * 1048576)
      case r3 of
        WorkExhausted w -> assertTrue "bounded" (w <= 4 * 1048576)
        WorkOk _ w -> assertTrue "bounded ok" (w <= 4 * 1048576)
      assertThrows "mutateSequence raises" (\(WorkCapExceeded w cap) -> cap == 250 && w == 200) $ do
        _ <- mutateSequenceWithCap 250 (B.replicate 100 97) 1 10 maxInputBytes
        pure ()
      assertEq "the per-call cap is 2^28" (2 ^ (28 :: Int)) maxSequenceWork
      assertEq "the corpus aggregate is 2^24" (2 ^ (24 :: Int)) corpusSequenceWork
  , test "the inverter forces the first SplitMix64 word" $
      forM_ [0, 1, 2, 3, 2 ^ (63 :: Int) - 2, 2 ^ (63 :: Int) - 1] $ \w -> do
        r <- newRng (seedFor w)
        got <- nextU64 r
        assertEq "first word" w (toInteger got)
  , test "bounded 7 rejects a first word below the threshold, keeps one equal to it" $ do
      -- 2^64 mod 7 = 2, so the threshold is (2^64 - 7) mod 7 = 2: raws 0 and 1 are redrawn
      assertEq "threshold" 2 (defaultThreshold 7)
      probe <- newRng (seedFor 0)
      _ <- nextU64 probe
      w1 <- nextU64 probe
      r <- newRng (seedFor 0)
      v <- bounded r 7
      assertEq "rejected the 0, answered from the second word" (toInteger w1 `mod` 7) v
      after <- rngState r
      afterProbe <- rngState probe
      assertEq "consumed two words" afterProbe after
      r2 <- newRng (seedFor 2)
      v2 <- bounded r2 7
      assertEq "a word equal to the threshold is kept" 2 v2
  , test "the rejection is visible through mutateSequence (real values, cross-checked with the Ruby lane)" $ do
      let input = hx "000102030405"
      a <- mutateSequence input (toInteger (seedFor 0)) 3 100
      b <- mutateSequence input (toInteger (seedFor 1)) 3 100
      c <- mutateSequence input (toInteger (seedFor 2)) 3 100
      d <- mutateSequence input (toInteger (seedFor 3)) 3 100
      assertEq "seed word 0" "fd030405" (hex a)
      assertEq "seed word 1" "ff043605" (hex b)
      assertEq "seed word 2" "000102b2020405" (hex c)
      assertEq "seed word 3" "00010203a2fa" (hex d)
  , test "the 2^63 + 1 threshold boundary" $ do
      let n = 2 ^ (63 :: Int) + 1 :: Integer
          threshold = 2 ^ (63 :: Int) - 1 :: Integer
      assertEq "threshold" threshold (toInteger (defaultThreshold (fromInteger n)))
      below <- newRng (seedFor (threshold - 1))
      probe <- newRng (seedFor (threshold - 1))
      _ <- nextU64 probe
      w1 <- nextU64 probe
      v <- bounded below n
      assertEq "below: redrawn" (toInteger w1 `mod` n) v
      at <- newRng (seedFor threshold)
      v2 <- bounded at n
      assertEq "equal: kept" threshold v2
  , test "bitmap, admission and the unreduced corpus_admit" $ do
      let b = record (record (record emptyBitmap 3) (3 + 65536)) (-1)
      assertEq "covered" [3, 65535] (coveredEdges b)
      assertTrue "has reduced" (hasEdge b 65539)
      assertFalse "has not" (hasEdge b 4)
      let (new1, n1, b1) = admit b [3, 4, 5, 5]
      assertEq "novel" (True, 2) (new1, n1)
      let (new2, n2, b2) = admit b1 [3, 4]
      assertEq "none" (False, 0) (new2, n2)
      assertEq "raw len" 8192 (B.length (rawBitmap b2))
      assertEq "raw bit 10" 4 (B.index (rawBitmap (record emptyBitmap 10)) 1)
      assertEq "count" 4 (count b2)
      assertEq "show" "Bitmap<edges=4>" (show b2)
      assertEq "admit" (True, 1) (corpusAdmit [1, 2, 3] [2, 4])
      assertEq "redundant" (False, 0) (corpusAdmit [1, 2, 3] [1, 2])
      assertEq "unique" (True, 1) (corpusAdmit [] [7, 7, 7])
      assertEq "unreduced" (True, 1) (corpusAdmit [1] [1 + 65536])
      assertEq "empty trace" (False, 0) (corpusAdmit [1] [])
  , test "power schedule rows, including exact half-way ties" $ do
      forM_ energyRows $ \(base, len, new, total, want) ->
        assertEq (show (base, len, new, total)) want (calculateEnergy base len new total)
      -- exact .5 ties round UP (banker's rounding would give 2, 2, 8, 2)
      assertEq "3,4" 2 (calculateEnergy 3 4 0 0)
      assertEq "5,4" 3 (calculateEnergy 5 4 0 0)
      assertEq "10,4,3,6" 8 (calculateEnergy 10 4 3 6)
      assertEq "25,100" 3 (calculateEnergy 25 100 0 0)
      assertEq "1,4" 1 (calculateEnergy 1 4 0 0)
      assertEq "len 0" 10 (calculateEnergy 10 0 0 0)
  , test "real sig_hash values match the Python oracle and the Ruby lane" $ do
      forM_ sigs $ \(k, fs, want) -> assertEq (T.unpack k) want (crashSignature k fs)
      assertEq "fnv abc" 16654208175385433931 (fnv1a64 "abc")
  , test "the signature is documented as unescaped; kinds are sanitised" $ do
      assertTrue "x:y" (duplicateCrash "k" ["x:y"] "k" ["x", "y"])
      assertFalse "a b" (duplicateCrash "k" ["a"] "k" ["b"])
      assertEq "sanitize" "a_b_c" (sanitizeKind "a:b c")
      assertEq "empty" "<unreadable>" (sanitizeKind "")
      assertEq "cap" 64 (T.length (sanitizeKind (T.replicate 500 "b")))
      assertEq "allowed" "Foo.bar<$>/-" (sanitizeKind "Foo.bar<$>/-")
  , test "the closed failure vocabulary" $ do
      assertEq "all" ["fuzz-crash", "fuzz-timeout", "fuzz-oom", "fuzz-leak", "fuzz-panic"] allFailureIds
      forM_ allFailureIds $ \i -> assertTrue "registered" (isRegistered i)
      forM_ ["fuzz-bogus", "FUZZ-CRASH", "fuzz-crash\n", "", " fuzz-crash"] $ \i -> assertFalse "unregistered" (isRegistered i)
  , test "bisection halves: a 9-byte crasher needing one byte tries chunks 4, 2, 1 in that order" $ do
      (probe, seen) <- traceProbe (B.elem 5)
      (out, spent, fin) <- minimize (hx "000102030405060708") sigK probe 1000
      assertEq "result" (B.pack [5]) out
      assertEq "spent" 8 spent
      assertTrue "finished" fin
      -- cross-checked with the Ruby lane: chunk 4 (delete 0..3 accepted, then 0..3
      -- of the rest rejected, then offset 4 accepted), chunk 2 (offset 0 rejected,
      -- offset 2 accepted), chunk 1 (offset 0 accepted, then the last byte
      -- rejected), then the zeroing pass tries 05 -> 00 (rejected)
      s <- seen
      assertEq "trace" ["0405060708", "08", "04050607", "0607", "0405", "05", "", "00"] s
  , test "bisection halves: a 1-byte crasher starts at chunk 1, so the empty input is reachable" $ do
      (probe, seen) <- traceProbe (const True)
      (out, spent, fin) <- minimize (hx "07") sigK probe 1000
      assertEq "empty" B.empty out
      assertEq "spent" 1 spent
      assertTrue "finished" fin
      s <- seen
      assertEq "trace" [""] s
  , test "bisection halves, then byte zeroing of non-essential bytes" $ do
      (probe, seen) <- traceProbe ((>= 2) . B.length)
      (out, spent, _) <- minimize (hx "01020304") sigK probe 1000
      assertEq "zeroed" (B.pack [0, 0]) out
      assertEq "spent" 6 spent
      s <- seen
      assertEq "trace" ["0304", "", "04", "03", "0004", "0000"] s
  , test "the budget counts rejected candidates too" $ do
      (probe, seen) <- traceProbe (B.elem 5)
      (out, spent, fin) <- minimize (hx "000102030405060708") sigK probe 3
      assertEq "result" "04050607" (hex out)
      assertEq "spent" 3 spent
      assertFalse "unfinished" fin
      s <- seen
      assertEq "trace" ["0405060708", "08", "04050607"] s
      (p2, seen2) <- traceProbe (const False)
      (_, spent2, fin2) <- minimize (B.replicate 64 9) sigK p2 4
      assertEq "all rejected" 4 spent2
      assertFalse "unfinished2" fin2
      s2 <- seen2
      assertEq "four probes" 4 (length s2)
      (p3, _) <- traceProbe (const True)
      z <- minimize B.empty sigK p3 0
      assertEq "empty" (B.empty, 0, True) z
      z2 <- minimize (B.pack [1]) sigK p3 0
      assertEq "zero budget" (B.pack [1], 0, False) z2
  , test "the minimiser keeps the exact signature: another crash is never accepted" $ do
      let probe c = pure (if B.length c >= 3 then Crash "K" ["f"] else Crash "OTHER" ["f"])
      (out, _, fin) <- minimize (hx "0a0b0c0d") sigK probe 1000
      assertEq "len" 3 (B.length out)
      assertTrue "finished" fin
  , test "minimizeCrashingInput is validated and bounded" $ do
      let probe _ = pure Pass
      o <- minimizeCrashingInput "abc" sigK 10 probe
      assertEq "unchanged" "abc" o
      assertThrows "negative" isArg (minimizeCrashingInput "abc" sigK (-1) probe)
      assertThrows "too big" isArg (minimizeCrashingInput "abc" sigK (maxMinimizeBudget + 1) probe)
      assertThrows "input" isArg (minimizeCrashingInput (B.replicate (maxInputBytes + 1) 97) sigK 1 probe)
  , test "tap format and closed ids" $ do
      forM_ allFailureIds $ \i -> do
        l <- formatTapFailure 7 "ct_compare" i "3a8f19b2"
        assertEq "line" ("not ok 7 - ct_compare [" <> i <> ":3a8f19b2]") l
      forM_ ["fuzz-bogus", "FUZZ-CRASH", "fuzz-crash\n", ""] $ \i ->
        assertThrows "id" isArg (formatTapFailure 1 "t" i "3a8f19b2")
      forM_ ["3A8F19B2", "3a8f19b", "3a8f19b22", "", "3a8f19b2\n", "zzzzzzzz"] $ \s ->
        assertThrows "sig" isArg (formatTapFailure 1 "t" "fuzz-crash" s)
      forM_ [0, 2 ^ (31 :: Int), -1] $ \n ->
        assertThrows "n" isArg (formatTapFailure n "t" "fuzz-crash" "3a8f19b2")
  , test "TAP target tokens are the VER06 9.3 byte grammar (values from the Ruby lane's format_tap_failure)" $ do
      let line t = formatTapFailure 3 t "fuzz-crash" "3a8f19b2"
          e = "\x00e9"
      l1 <- line "a#b"
      assertEq "hash" "not ok 3 - a_b [fuzz-crash:3a8f19b2]" l1
      l2 <- line "a b c"
      assertEq "space" "not ok 3 - a_b_c [fuzz-crash:3a8f19b2]" l2
      l3 <- line "a\nb"
      assertEq "lf" "not ok 3 - a_b [fuzz-crash:3a8f19b2]" l3
      l4 <- line "a\rb"
      assertEq "cr" "not ok 3 - a_b [fuzz-crash:3a8f19b2]" l4
      l5 <- line (e <> "\x202e" <> "x")
      assertEq "bidi" "not ok 3 - _____x [fuzz-crash:3a8f19b2]" l5
      l6 <- line (T.replicate 200 "x")
      assertEq "long" ("not ok 3 - " <> T.replicate 128 "x" <> " [fuzz-crash:3a8f19b2]") l6
      l7 <- line (T.replicate 127 "x" <> e <> "yyyyy")
      assertEq "cut mid-character" ("not ok 3 - " <> T.replicate 127 "x" <> "_ [fuzz-crash:3a8f19b2]") l7
      l8 <- line (T.replicate 126 "x" <> e <> "zz")
      assertEq "whole character at the edge" ("not ok 3 - " <> T.replicate 126 "x" <> "__ [fuzz-crash:3a8f19b2]") l8
      l9 <- line (T.replicate 70 e)
      assertEq "all multibyte" ("not ok 3 - " <> T.replicate 128 "_" <> " [fuzz-crash:3a8f19b2]") l9
      l10 <- line ""
      assertEq "empty" "not ok 3 -  [fuzz-crash:3a8f19b2]" l10
      l11 <- line "ct_compare.v1/x-y"
      assertEq "grammar kept" "not ok 3 - ct_compare.v1/x-y [fuzz-crash:3a8f19b2]" l11
      assertEq "bytes" "a_b" (tapTokenBytes "a#b")
      assertEq "token" "a_b" (tapToken "a#b")
      assertEq "bytes of an invalid UTF-8 target" "__" (tapTokenBytes (B.pack [0xFF, 0xFE]))
  , test "tapFailures for a report" $ do
      let crash = CrashRecord "4eca7ae8" "panic" ["eval", "step"] "x" 2 True False
          report = FuzzReport 5 1 0 [crash] 1 2 0 0 tierBlackBox 7 ["a"] 0
      ls <- tapFailures report "t" 3
      assertEq "lines" ["not ok 3 - t [fuzz-crash:4eca7ae8]", "not ok 4 - t [fuzz-timeout:4e61ea76]", "not ok 5 - t [fuzz-oom:7146a410]"] ls
      let clean = FuzzReport 5 1 0 [] 0 0 0 0 tierBlackBox 7 [] 0
      none <- tapFailures clean "t" 1
      assertEq "clean" [] none
      assertTrue "isClean" (isClean clean)
      assertFalse "not clean" (isClean report)
      assertEq "unique" 1 (uniqueCrashes report)
      assertFalse "zero iterations" (isClean clean {rIterations = 0})
      assertFalse "dropped" (isClean clean {rCrashesDropped = 1})
      assertFalse "admission dropped" (isClean clean {rAdmissionsDropped = 1})
      assertEq "report show hides data" "FuzzReport<iterations=5 seeds=1 edges=0 crashes=1 timeouts=1 ooms=2>" (show report)
      assertEq "crash show hides input" "CrashRecord<\"4eca7ae8\" \"panic\" hits=2>" (show crash)
  , test "options are validated, and show is opaque" $ do
      validateFuzzOptions defaultFuzzOptions
      let bad f = assertThrows "bad option" isArg (validateFuzzOptions (f defaultFuzzOptions))
      bad (\o -> o {fzMaxIterations = -1})
      bad (\o -> o {fzMaxIterations = maxIterations + 1})
      bad (\o -> o {fzMaxInputLen = maxInputBytes + 1})
      bad (\o -> o {fzBaseEnergy = -1})
      bad (\o -> o {fzMinimizeBudget = maxMinimizeBudget + 1})
      bad (\o -> o {fzStepBudget = maxStepBudget + 1})
      bad (\o -> o {fzMemBudget = maxMemBudget + 1})
      bad (\o -> o {fzTimeoutMs = 0})
      bad (\o -> o {fzTimeoutMs = maxTimeoutMs + 1})
      assertEq "show" "FuzzOptions<max_iterations=1000>" (show defaultFuzzOptions)
  , test "kindOf reads the exception TYPE only, never the message" $ do
      r <- try (ioError (userError "secret payload")) :: IO (Either SomeException ())
      case r of
        Left e -> assertEq "type" "IOException" (kindOf e)
        Right _ -> failTest "expected an exception"
  , test "endian text and derived instances" $ do
      assertEq "little" (Just Little) (parseEndian "little")
      assertEq "big" (Just Big) (parseEndian "big")
      assertEq "other" Nothing (parseEndian "Little")
      assertEq "show" "Little" (show Little)
      assertEq "seq show" "WorkExhausted 3" (show (WorkExhausted 3))
      assertEq "wce show" "WorkCapExceeded {wceWork = 1, wceCap = 2}" (show (WorkCapExceeded 1 2))
  , test "the vector replayer rejects over-long or non-numeric tokens before converting them" $ do
      a <- runVector (Vector "bit_flip" ["00", '1' : replicate 5000 '9', "1"] "00")
      assertLeft "huge integer" a
      b <- runVector (Vector "bit_flip" ["00", "1x", "1"] "00")
      assertLeft "non-numeric" b
      c <- runVector (Vector "bit_flip" ["0g", "1", "1"] "00")
      assertLeft "bad hex" c
      d <- runVector (Vector "bit_flip" ["00", "1"] "00")
      assertLeft "arity" d
      e <- runVector (Vector "bit_flip" ["00", "0", "1"] "01")
      assertEq "good row" (Right ()) e
  ]

