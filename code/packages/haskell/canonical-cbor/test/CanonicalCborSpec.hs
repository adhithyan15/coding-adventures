{-# LANGUAGE OverloadedStrings #-}
module CanonicalCborSpec
  ( runCanonicalCborTests
  ) where

import Control.Monad (when)
import Data.Bits ((.&.), shiftL, shiftR, xor)
import qualified Data.ByteString as BS
import qualified Data.List as List
import qualified Data.Text as T
import Data.Word (Word8, Word64)
import System.IO (hPutStrLn, stderr)

import CodingAdventures.CanonicalCbor
import SimpleJson (bytesToHex, hexToBytes)

runCanonicalCborTests :: IO Int
runCanonicalCborTests = do
  let tests :: [(String, IO ())]
      tests =
        [ ("cve1-word64-wraparound", testCve1Word64Wraparound)
        , ("cve2-allocation-bomb", testCve2AllocationBomb)
        , ("cve3-recursion-depth-bomb", testCve3RecursionDepthBomb)
        , ("cve4-canonical-map-sorting-and-duplicates", testCve4MapSortingAndDuplicates)
        , ("cve5-spanned-map-redaction", testCve5SpannedMapRedaction)
        , ("cve6-invalid-utf8-rejection", testCve6InvalidUtf8)
        , ("cve7-structural-sharing-defense", testCve7StructuralSharingDefense)
        , ("decode-map-spans", testDecodeMapSpans)
        , ("messages-and-constants", testMessagesAndConstants)
        , ("integer-boundaries", testIntegerBoundaries)
        , ("property-roundtrip-and-invariance", testPropertyRoundtripAndInvariance)
        , ("adversarial-mutation-fuzzing", testMutationFuzzing)
        ]
  runAll 0 tests
  where
    runAll count [] = return count
    runAll count ((_, act):rest) = do
      _ <- act
      runAll (count + 1) rest

-- ============================================================================
-- CVE-1: Word64 -> Signed Int Wraparound
-- ============================================================================

testCve1Word64Wraparound :: IO ()
testCve1Word64Wraparound = do
  -- 0x5b followed by 2^63 + 1 (0x8000000000000001)
  -- In signed 64-bit Int, this wraps to -9223372036854775807.
  let wireBytes = BS.pack [0x5b, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01]
  case decode wireBytes of
    Left err -> assertEqual "cve1 bytes" LengthTooLarge (cborErrorId err)
    Right _  -> failTest "cve1-wraparound" "expected LengthTooLarge for wrapped Word64 byte string"

  let wireText = BS.pack [0x7b, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01]
  case decode wireText of
    Left err -> assertEqual "cve1 text" LengthTooLarge (cborErrorId err)
    Right _  -> failTest "cve1-wraparound" "expected LengthTooLarge for wrapped Word64 text string"

  let wireArray = BS.pack [0x9b, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01]
  case decode wireArray of
    Left err -> assertEqual "cve1 array" LengthTooLarge (cborErrorId err)
    Right _  -> failTest "cve1-wraparound" "expected LengthTooLarge for wrapped Word64 array"

  let wireMap = BS.pack [0xbb, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01]
  case decode wireMap of
    Left err -> assertEqual "cve1 map" LengthTooLarge (cborErrorId err)
    Right _  -> failTest "cve1-wraparound" "expected LengthTooLarge for wrapped Word64 map"

-- ============================================================================
-- CVE-2: High Length Allocation Bomb
-- ============================================================================

testCve2AllocationBomb :: IO ()
testCve2AllocationBomb = do
  -- 2^63 - 1 length with 0 bytes following
  let bombBytes = BS.pack [0x5b, 0x7f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]
  case decode bombBytes of
    Left err -> assertEqual "cve2 bytes" LengthTooLarge (cborErrorId err)
    Right _  -> failTest "cve2-allocation-bomb" "expected LengthTooLarge for allocation bomb"

  let bombArray = BS.pack [0x9b, 0x7f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]
  case decode bombArray of
    Left err -> assertEqual "cve2 array" LengthTooLarge (cborErrorId err)
    Right _  -> failTest "cve2-allocation-bomb" "expected LengthTooLarge for array allocation bomb"

  let bombMap = BS.pack [0xbb, 0x7f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]
  case decode bombMap of
    Left err -> assertEqual "cve2 map" LengthTooLarge (cborErrorId err)
    Right _  -> failTest "cve2-allocation-bomb" "expected LengthTooLarge for map allocation bomb"

-- ============================================================================
-- CVE-3: Recursion Depth Bomb (Depth 128 succeeds, 129 fails)
-- ============================================================================

testCve3RecursionDepthBomb :: IO ()
testCve3RecursionDepthBomb = do
  let nest128 = nestArray 128 (CborUnsigned 0)
      wire128 = BS.replicate 128 0x81 `BS.append` BS.singleton 0x00
  case encodeChecked nest128 of
    Right enc -> assertEqual "nest128 wire" wire128 enc
    Left err  -> failTest "cve3" ("encodeChecked 128 failed: " ++ show err)

  case decode wire128 of
    Right val -> assertEqual "nest128 val" nest128 val
    Left err  -> failTest "cve3" ("decode 128 failed: " ++ show err)

  let nest129 = nestArray 129 (CborUnsigned 0)
      wire129 = BS.replicate 129 0x81 `BS.append` BS.singleton 0x00
  case encodeChecked nest129 of
    Left err -> assertEqual "nest129 encode error" EncodeTooDeep (cborErrorId err)
    Right _  -> failTest "cve3" "expected EncodeTooDeep for depth 129"

  case decode wire129 of
    Left err -> assertEqual "nest129 decode error" TooDeep (cborErrorId err)
    Right _  -> failTest "cve3" "expected TooDeep for wire depth 129"

  -- Tag depth test
  let tag128 = nestTag 128 (CborUnsigned 0)
  case encodeChecked tag128 of
    Right _  -> return ()
    Left err -> failTest "cve3 tag128" ("tag128 encode failed: " ++ show err)

  let tag129 = nestTag 129 (CborUnsigned 0)
  case encodeChecked tag129 of
    Left err -> assertEqual "tag129 encode error" EncodeTooDeep (cborErrorId err)
    Right _  -> failTest "cve3 tag129" "expected EncodeTooDeep for tag 129"

nestArray :: Int -> CborValue -> CborValue
nestArray n leaf = List.foldl' (\acc _ -> CborArray [acc]) leaf [1..n]

nestTag :: Int -> CborValue -> CborValue
nestTag n leaf = List.foldl' (\acc _ -> CborTag 1 acc) leaf [1..n]

-- ============================================================================
-- CVE-4: Canonical Map Sorting & Duplicate Keys
-- ============================================================================

testCve4MapSortingAndDuplicates :: IO ()
testCve4MapSortingAndDuplicates = do
  -- Shorter length must precede longer length regardless of first byte:
  -- Key 0x01 (1 byte) vs Key 0x1818 (2 bytes). 0x18 > 0x01, but 0x01 is shorter!
  let entry1 = CborMapEntry (CborUnsigned 1) (CborText "one")
      entry2 = CborMapEntry (CborUnsigned 24) (CborText "twenty-four")
      m1 = CborMap [entry2, entry1]
      m2 = CborMap [entry1, entry2]

  case (encodeChecked m1, encodeChecked m2) of
    (Right b1, Right b2) -> do
      assertEqual "map sorting invariance" b1 b2
      -- Verify key 1 precedes key 24 on wire
      let expectedHex = "a201636f6e6518186b7477656e74792d666f7572"
      assertEqual "canonical wire map" expectedHex (bytesToHex b1)
    (Left e1, _) -> failTest "cve4" ("encode m1 failed: " ++ show e1)
    (_, Left e2) -> failTest "cve4" ("encode m2 failed: " ++ show e2)

  -- Out-of-order map wire rejected
  let badWire = hexToBytes "a218186b7477656e74792d666f757201636f6e65"
  case decode badWire of
    Left err -> assertEqual "map out of order" NonCanonicalMapOrder (cborErrorId err)
    Right _  -> failTest "cve4" "expected NonCanonicalMapOrder for out of order keys"

  -- Duplicate keys rejected during encode
  let dupMap = CborMap [CborMapEntry (CborText "k") (CborUnsigned 1), CborMapEntry (CborText "k") (CborUnsigned 2)]
  case encodeChecked dupMap of
    Left err -> assertEqual "dup encode" DuplicateMapKey (cborErrorId err)
    Right _  -> failTest "cve4" "expected DuplicateMapKey during encode"

  -- Duplicate keys rejected during decode with NonCanonicalMapOrder (strict monotonicity)
  let dupWire = hexToBytes "a2616b01616b02"
  case decode dupWire of
    Left err -> assertEqual "dup decode" NonCanonicalMapOrder (cborErrorId err)
    Right _  -> failTest "cve4" "expected NonCanonicalMapOrder during decode"

  -- Empty map and 1-item map
  case encodeChecked (CborMap []) of
    Right b -> assertEqual "empty map" (hexToBytes "a0") b
    Left e  -> failTest "empty map" (show e)

  case encodeChecked (CborMap [CborMapEntry (CborUnsigned 1) (CborUnsigned 2)]) of
    Right b -> assertEqual "1-item map" (hexToBytes "a10102") b
    Left e  -> failTest "1-item map" (show e)

-- ============================================================================
-- CVE-5: SpannedMapEntry Redaction (Standing Rule #20)
-- ============================================================================

testCve5SpannedMapRedaction :: IO ()
testCve5SpannedMapRedaction = do
  let secretKey = CborText "SECRET_KEY_SUPER_CONFIDENTIAL_123"
      secretVal = CborText "SECRET_PASSWORD_987654321"
      spanVal = Span 12 45
      entry = SpannedMapEntry secretKey secretVal spanVal
      rendered = show entry

  when ("SECRET_KEY_SUPER_CONFIDENTIAL_123" `List.isInfixOf` rendered) $
    failTest "cve5" "SpannedMapEntry show leaked confidential secret key!"

  when ("SECRET_PASSWORD_987654321" `List.isInfixOf` rendered) $
    failTest "cve5" "SpannedMapEntry show leaked confidential secret value!"

  when (not (T.unpack redactedPlaceholder `List.isInfixOf` rendered)) $
    failTest "cve5" "SpannedMapEntry show missing <redacted> placeholder!"

  -- Test Span constructor and helpers
  assertEqual "spanLength" 33 (spanLength spanVal)
  assertEqual "mkSpan valid" (Just (Span 10 20)) (mkSpan 10 20)
  assertEqual "mkSpan invalid" Nothing (mkSpan 20 10)
  assertEqual "show Span" "[12, 45)" (show spanVal)

-- ============================================================================
-- CVE-6: Invalid UTF-8 Rejection
-- ============================================================================

testCve6InvalidUtf8 :: IO ()
testCve6InvalidUtf8 = do
  -- 0x61 0xff: single invalid byte in text string
  case decode (hexToBytes "61ff") of
    Left err -> assertEqual "invalid utf8 single byte" InvalidUtf8 (cborErrorId err)
    Right _  -> failTest "cve6" "expected InvalidUtf8"

  -- 0x62 0xc0 0x80: overlong 2-byte NUL
  case decode (hexToBytes "62c080") of
    Left err -> assertEqual "overlong nul utf8" InvalidUtf8 (cborErrorId err)
    Right _  -> failTest "cve6" "expected InvalidUtf8"

  -- 0x64 0xeda080 0x00: surrogate half
  case decode (BS.pack [0x63, 0xed, 0xa0, 0x80]) of
    Left err -> assertEqual "surrogate utf8" InvalidUtf8 (cborErrorId err)
    Right _  -> failTest "cve6" "expected InvalidUtf8"

-- ============================================================================
-- CVE-7: Structural Sharing Allocation Defense
-- ============================================================================

testCve7StructuralSharingDefense :: IO ()
testCve7StructuralSharingDefense = do
  -- 1,000 shared references to a 10,000-byte binary item (total > 10 MB > 1 MB limit)
  let sharedItem = CborBytes (BS.replicate 10000 0x5a)
      hugeArray = CborArray (replicate 1000 sharedItem)
  case encodeChecked hugeArray of
    Left err -> assertEqual "cve7 encode array" EncodeTooLarge (cborErrorId err)
    Right _  -> failTest "cve7" "expected EncodeTooLarge for structurally shared array"

  case encodeIntoChecked hugeArray (BS.pack [1, 2, 3]) of
    Left err -> assertEqual "cve7 encodeInto array" EncodeTooLarge (cborErrorId err)
    Right _  -> failTest "cve7" "expected EncodeTooLarge for encodeIntoChecked array"

  -- Test encodeIntoChecked with prefix exceeding maxEncodedBytes
  let largePrefix = BS.replicate 1048577 0x00
  case encodeIntoChecked (CborUnsigned 1) largePrefix of
    Left err -> assertEqual "encodeInto large prefix" EncodeTooLarge (cborErrorId err)
    Right _  -> failTest "cve7" "expected EncodeTooLarge when prefix exceeds limit"

  -- Test encodeIntoChecked successful append
  let initialPrefix = BS.pack [0xaa, 0xbb]
  case encodeIntoChecked (CborUnsigned 42) initialPrefix of
    Left err -> failTest "encodeInto success" ("failed: " ++ show err)
    Right res -> assertEqual "encodeInto success" (BS.pack [0xaa, 0xbb, 0x18, 0x2a]) res

  -- Unbounded lazy spine rejection without hanging or OOM (CVE DoS defense)
  let infiniteArray = CborArray (repeat (CborUnsigned 0))
  case encodeChecked infiniteArray of
    Left err -> assertEqual "infinite array encode" EncodeTooLarge (cborErrorId err)
    Right _  -> failTest "cve7" "expected EncodeTooLarge for infinite array"

  -- Multi-large-key map bomb (each key 900 KB, total > 1 MiB budget)
  let largeKey1 = CborBytes (BS.replicate 900000 0x01)
      largeKey2 = CborBytes (BS.replicate 900000 0x02)
      multiKeyMap = CborMap [CborMapEntry largeKey1 (CborUnsigned 1), CborMapEntry largeKey2 (CborUnsigned 2)]
  case encodeChecked multiKeyMap of
    Left err -> assertEqual "multi large key map" EncodeTooLarge (cborErrorId err)
    Right _  -> failTest "cve7" "expected EncodeTooLarge for multi large key map"

-- ============================================================================
-- decodeMapSpans Coverage
-- ============================================================================

testDecodeMapSpans :: IO ()
testDecodeMapSpans = do
  -- Valid map: {"a": 1, "b": [3, 4]} -> "a26161016162820304"
  let wire = hexToBytes "a26161016162820304"
  case decodeMapSpans wire of
    Left err -> failTest "decodeMapSpans" ("unexpected error: " ++ show err)
    Right DecodedNonMap -> failTest "decodeMapSpans" "expected DecodedMap, got DecodedNonMap"
    Right (DecodedMap entries) -> do
      assertEqual "entries count" 2 (List.length entries)
      let e1 = entries !! 0
          e2 = entries !! 1
      assertEqual "e1 key" (CborText "a") (spannedKey e1)
      assertEqual "e1 val" (CborUnsigned 1) (spannedVal e1)
      assertEqual "e1 span" (Span 3 4) (spannedValueSpan e1)
      assertEqual "e1 length" 1 (spanLength (spannedValueSpan e1))
      assertEqual "e1 wire slice" (BS.take 1 (BS.drop 3 wire)) (hexToBytes "01")

      assertEqual "e2 key" (CborText "b") (spannedKey e2)
      assertEqual "e2 val" (CborArray [CborUnsigned 3, CborUnsigned 4]) (spannedVal e2)
      assertEqual "e2 span" (Span 6 9) (spannedValueSpan e2)
      assertEqual "e2 length" 3 (spanLength (spannedValueSpan e2))
      assertEqual "e2 wire slice" (BS.take 3 (BS.drop 6 wire)) (hexToBytes "820304")

  -- Non-map items return DecodedNonMap
  case decodeMapSpans (hexToBytes "01") of
    Right DecodedNonMap -> return ()
    other -> failTest "decodeMapSpans non-map" ("expected DecodedNonMap, got " ++ show other)

  case decodeMapSpans (hexToBytes "820102") of
    Right DecodedNonMap -> return ()
    other -> failTest "decodeMapSpans non-map array" ("expected DecodedNonMap, got " ++ show other)

  -- Non-map item with trailing bytes fails with TrailingBytes
  case decodeMapSpans (hexToBytes "01ff") of
    Left err -> assertEqual "non-map trailing bytes" TrailingBytes (cborErrorId err)
    other    -> failTest "decodeMapSpans non-map trailing" ("expected TrailingBytes, got " ++ show other)

  -- Map item with trailing bytes fails with TrailingBytes
  case decodeMapSpans (wire `BS.snoc` 0x00) of
    Left err -> assertEqual "map trailing bytes" TrailingBytes (cborErrorId err)
    other    -> failTest "decodeMapSpans map trailing" ("expected TrailingBytes, got " ++ show other)

  -- Invalid wire returns error
  case decodeMapSpans BS.empty of
    Left err -> assertEqual "empty decodeMapSpans" UnexpectedEof (cborErrorId err)
    other    -> failTest "decodeMapSpans empty" ("expected UnexpectedEof, got " ++ show other)

  case decodeMapSpans (hexToBytes "bf616101ff") of
    Left err -> assertEqual "indefinite decodeMapSpans" Indefinite (cborErrorId err)
    other    -> failTest "decodeMapSpans indef" ("expected Indefinite, got " ++ show other)

  case decodeMapSpans (hexToBytes "b800") of
    Left err -> assertEqual "nonminimal decodeMapSpans" NonMinimalInteger (cborErrorId err)
    other    -> failTest "decodeMapSpans nonminimal" ("expected NonMinimalInteger, got " ++ show other)

  case decodeMapSpans (hexToBytes "bc") of
    Left err -> assertEqual "reserved decodeMapSpans" Reserved (cborErrorId err)
    other    -> failTest "decodeMapSpans reserved" ("expected Reserved, got " ++ show other)

-- ============================================================================
-- Messages, Constants & Types Coverage
-- ============================================================================

testMessagesAndConstants :: IO ()
testMessagesAndConstants = do
  assertEqual "maxNestingDepth" 128 maxNestingDepth
  assertEqual "maxEncodedBytes" 1048576 maxEncodedBytes
  assertEqual "redactedPlaceholder" "<redacted>" redactedPlaceholder
  assertEqual "invalidPlaceholder" "<invalid>" invalidPlaceholder
  assertEqual "unknownErrorIdMessage" "canonical-cbor: unknown error id" unknownErrorIdMessage

  -- Verify all 14 error ids have bijective text and matching messages
  assertEqual "allCborErrorIds length" 14 (List.length allCborErrorIds)
  assertEqual "allDiagnosticMessages length" 14 (List.length allDiagnosticMessages)

  mapM_ checkErrorId allCborErrorIds

  -- Test unknown text fallback
  assertEqual "unknown textToId" UnknownErrorId (textToId "some-random-id")
  assertEqual "unknown idToText" "unknown" (idToText UnknownErrorId)
  assertEqual "unknown messageFor" "canonical-cbor: unknown error id" (messageFor UnknownErrorId)

  -- Test mkCborError
  let err = mkCborError UnexpectedEof
  assertEqual "mkCborError id" UnexpectedEof (cborErrorId err)
  assertEqual "mkCborError msg" (messageFor UnexpectedEof) (cborErrorMessage err)

  -- Field selectors and Show instances
  let sp = Span 10 20
  assertEqual "spanStart" 10 (spanStart sp)
  assertEqual "spanFinish" 20 (spanFinish sp)

  let entry = CborMapEntry (CborUnsigned 1) CborNull
  assertEqual "cborMapKey" (CborUnsigned 1) (cborMapKey entry)
  assertEqual "cborMapVal" CborNull (cborMapVal entry)

  when (not ("UnexpectedEof" `List.isInfixOf` show (mkCborError UnexpectedEof))) $
    failTest "show CborError" "show CborError failed"

  when (not ("CborMapEntry" `List.isInfixOf` show entry)) $
    failTest "show CborMapEntry" "show CborMapEntry failed"

  when (not ("DecodedMap" `List.isInfixOf` show (DecodedMap []))) $
    failTest "show DecodedMap" "show DecodedMap failed"

  when (not ("DecodedNonMap" `List.isInfixOf` show DecodedNonMap)) $
    failTest "show DecodedNonMap" "show DecodedNonMap failed"

  when (not ("CborBool" `List.isInfixOf` show (CborBool True))) $
    failTest "show CborBool" "show CborBool failed"
  where
    checkErrorId eid = do
      let txt = idToText eid
          eid' = textToId txt
          msg = messageFor eid
      assertEqual ("idToText/textToId roundtrip: " ++ show eid) eid eid'
      if not ("canonical-cbor: " `T.isPrefixOf` msg)
        then failTest "message prefix" ("message for " ++ show eid ++ " lacks prefix: " ++ T.unpack msg)
        else return ()

-- ============================================================================
-- Integer Boundaries
-- ============================================================================

testIntegerBoundaries :: IO ()
testIntegerBoundaries = do
  let unsignedCases =
        [ (0, "00")
        , (23, "17")
        , (24, "1818")
        , (255, "18ff")
        , (256, "190100")
        , (65535, "19ffff")
        , (65536, "1a00010000")
        , (4294967295, "1affffffff")
        , (4294967296, "1b0000000100000000")
        , (18446744073709551615, "1bffffffffffffffff")
        ]
  mapM_ checkUnsigned unsignedCases

  let negativeCases =
        [ (0, "20")                    -- -1
        , (23, "37")                   -- -24
        , (24, "3818")                 -- -25
        , (255, "38ff")                -- -256
        , (256, "390100")              -- -257
        , (65535, "39ffff")            -- -65536
        , (4294967295, "3affffffff")   -- -4294967296
        , (18446744073709551615, "3bffffffffffffffff") -- -18446744073709551616
        ]
  mapM_ checkNegative negativeCases
  where
    checkUnsigned (n, hexExpected) = do
      let val = CborUnsigned n
          bytes = hexToBytes hexExpected
      case encodeChecked val of
        Left err -> failTest "unsigned encode" ("failed for " ++ show n ++ ": " ++ show err)
        Right b  -> assertEqual ("unsigned " ++ show n) bytes b
      case decode bytes of
        Left err  -> failTest "unsigned decode" ("failed for " ++ show n ++ ": " ++ show err)
        Right val' -> assertEqual ("unsigned val " ++ show n) val val'

    checkNegative (n, hexExpected) = do
      let val = CborNegative n
          bytes = hexToBytes hexExpected
      case encodeChecked val of
        Left err -> failTest "negative encode" ("failed for " ++ show n ++ ": " ++ show err)
        Right b  -> assertEqual ("negative " ++ show n) bytes b
      case decode bytes of
        Left err  -> failTest "negative decode" ("failed for " ++ show n ++ ": " ++ show err)
        Right val' -> assertEqual ("negative val " ++ show n) val val'

-- ============================================================================
-- Property Tests: 500 Deterministic Round-trips & Permutation Invariance
-- ============================================================================

testPropertyRoundtripAndInvariance :: IO ()
testPropertyRoundtripAndInvariance = do
  runPropertyRoundtrips 500 0x123456789abcdef0
  runMapPermutations 50 0xfeedfacecafebabe

runPropertyRoundtrips :: Int -> Word64 -> IO ()
runPropertyRoundtrips 0 _ = return ()
runPropertyRoundtrips count seed = do
  let (val, seed') = genCborValue 0 seed
  case encodeChecked val of
    Left err -> failTest "property-roundtrip" ("encodeChecked failed: " ++ show err)
    Right enc ->
      case decode enc of
        Left err -> failTest "property-roundtrip" ("decode failed: " ++ show err)
        Right dec -> do
          assertEqual ("property roundtrip #" ++ show count) val dec
          runPropertyRoundtrips (count - 1) seed'

runMapPermutations :: Int -> Word64 -> IO ()
runMapPermutations 0 _ = return ()
runMapPermutations count seed = do
  let (entries, seed1) = genUniqueEntries 5 seed
      perm1 = entries
      perm2 = reverse entries
      perm3 = rotateLeft entries
  case (encodeChecked (CborMap perm1), encodeChecked (CborMap perm2), encodeChecked (CborMap perm3)) of
    (Right b1, Right b2, Right b3) -> do
      assertEqual ("map perm 1 vs 2 #" ++ show count) b1 b2
      assertEqual ("map perm 1 vs 3 #" ++ show count) b1 b3
      runMapPermutations (count - 1) seed1
    (Left e1, _, _) -> failTest "permutation" ("perm1 failed: " ++ show e1)
    (_, Left e2, _) -> failTest "permutation" ("perm2 failed: " ++ show e2)
    (_, _, Left e3) -> failTest "permutation" ("perm3 failed: " ++ show e3)
  where
    rotateLeft [] = []
    rotateLeft (x:xs) = xs ++ [x]

-- Generate random CborValue with bounded depth
genCborValue :: Int -> Word64 -> (CborValue, Word64)
genCborValue depth s
  | depth >= 4 =
      let (b, s1) = nextWord64 s
      in (CborUnsigned (b .&. 0xffff), s1)
  | otherwise =
      let (choice, s1) = nextWord64 s
          kind = choice `mod` 9
      in case kind of
           0 -> (CborUnsigned (choice .&. 0xffffff), s1)
           1 -> (CborNegative (choice .&. 0xffffff), s1)
           2 ->
             let len = fromIntegral ((choice `shiftR` 8) `mod` 16)
                 (bs, s2) = genBytes len s1
             in (CborBytes bs, s2)
           3 ->
             let len = fromIntegral ((choice `shiftR` 8) `mod` 8)
                 (txt, s2) = genText len s1
             in (CborText txt, s2)
           4 ->
             let len = fromIntegral ((choice `shiftR` 8) `mod` 4)
                 (items, s2) = genArrayItems len (depth + 1) s1
             in (CborArray items, s2)
           5 ->
             let (entries, s2) = genUniqueEntries 3 s1
             in (CborMap entries, s2)
           6 ->
             let (inner, s2) = genCborValue (depth + 1) s1
                 tagNum = (choice `shiftR` 8) `mod` 1000
             in (CborTag tagNum inner, s2)
           7 ->
             (CborBool (choice .&. 1 == 1), s1)
           _ ->
             (CborNull, s1)

genBytes :: Int -> Word64 -> (BS.ByteString, Word64)
genBytes 0 s = (BS.empty, s)
genBytes n s =
  let (w, s1) = nextWord64 s
      b = fromIntegral (w .&. 0xff) :: Word8
      (rest, s2) = genBytes (n - 1) s1
  in (BS.cons b rest, s2)

genText :: Int -> Word64 -> (T.Text, Word64)
genText 0 s = (T.empty, s)
genText n s =
  let (w, s1) = nextWord64 s
      char = ['a'..'z'] !! fromIntegral (w `mod` 26)
      (rest, s2) = genText (n - 1) s1
  in (T.cons char rest, s2)

genArrayItems :: Int -> Int -> Word64 -> ([CborValue], Word64)
genArrayItems 0 _ s = ([], s)
genArrayItems n depth s =
  let (item, s1) = genCborValue depth s
      (rest, s2) = genArrayItems (n - 1) depth s1
  in (item : rest, s2)

genUniqueEntries :: Int -> Word64 -> ([CborMapEntry], Word64)
genUniqueEntries count seed = go count (0 :: Word64) seed []
  where
    go :: Int -> Word64 -> Word64 -> [CborMapEntry] -> ([CborMapEntry], Word64)
    go 0 _ s acc = (reverse acc, s)
    go n idx s acc =
      let (val, s1) = genCborValue 2 s
          k = CborUnsigned idx
          entry = CborMapEntry k val
      in go (n - 1) (idx + 1) s1 (entry : acc)

nextWord64 :: Word64 -> (Word64, Word64)
nextWord64 s =
  let s1 = s `xor` (s `shiftL` 13)
      s2 = s1 `xor` (s1 `shiftR` 7)
      s3 = s2 `xor` (s2 `shiftL` 17)
  in (s3, s3)

-- ============================================================================
-- Adversarial Mutation Fuzzing (10,000 iterations)
-- ============================================================================

testMutationFuzzing :: IO ()
testMutationFuzzing = do
  -- Seed with valid CBOR samples
  let seeds =
        [ hexToBytes "00"
        , hexToBytes "18ff"
        , hexToBytes "20"
        , hexToBytes "430001ff"
        , hexToBytes "63e29883"
        , hexToBytes "8300f5f6"
        , hexToBytes "a201636f6e65026374776f"
        , hexToBytes "c0f6"
        , hexToBytes "f4"
        ]
  fuzzLoop 10000 0xdeadbeefcafebabe seeds

fuzzLoop :: Int -> Word64 -> [BS.ByteString] -> IO ()
fuzzLoop 0 _ _ = return ()
fuzzLoop iter prng samples = do
  let (r1, prng1) = nextWord64 prng
      sampleIdx = fromIntegral (r1 `mod` fromIntegral (List.length samples))
      sample = samples !! sampleIdx
      (mutated, prng2) = mutateByteString sample prng1

  case decode mutated of
    Right val ->
      -- Unconditionally require canonical re-encoding and decode roundtrip
      case encodeChecked val of
        Left err -> failTest "fuzz" ("re-encode of decoded value failed: " ++ show err)
        Right enc -> do
          assertEqual "fuzz canonical roundtrip" mutated enc
          case decode enc of
            Right val2 -> assertEqual "fuzz roundtrip value" val val2
            Left err   -> failTest "fuzz" ("re-decode of canonical wire failed: " ++ show err)

    Left err -> do
      -- Validate error invariants
      let eid = cborErrorId err
      if not (eid `elem` allCborErrorIds)
        then failTest "fuzz" ("invalid error id: " ++ show eid)
        else if not ("canonical-cbor: " `T.isPrefixOf` cborErrorMessage err)
          then failTest "fuzz" ("error message missing prefix: " ++ T.unpack (cborErrorMessage err))
          else return ()

  fuzzLoop (iter - 1) prng2 samples

mutateByteString :: BS.ByteString -> Word64 -> (BS.ByteString, Word64)
mutateByteString bs seed
  | BS.null bs = (BS.singleton 0x00, seed)
  | otherwise =
      let (strategy, s1) = nextWord64 seed
          (posR, s2) = nextWord64 s1
          pos = fromIntegral (posR `mod` fromIntegral (BS.length bs))
          (byteR, s3) = nextWord64 s2
          byteVal = fromIntegral (byteR .&. 0xff) :: Word8
      in case strategy `mod` 4 of
           0 -> -- Bit flip
             let flipped = BS.index bs pos `xor` (1 `shiftL` fromIntegral (byteR `mod` 8))
                 (prefix, suffix) = BS.splitAt pos bs
             in (prefix `BS.append` BS.cons flipped (BS.drop 1 suffix), s3)
           1 -> -- Byte truncation
             (BS.take pos bs, s3)
           2 -> -- Byte insertion
             let (prefix, suffix) = BS.splitAt pos bs
             in (prefix `BS.append` BS.cons byteVal suffix, s3)
           _ -> -- Random byte replacement
             let (prefix, suffix) = BS.splitAt pos bs
             in (prefix `BS.append` BS.cons byteVal (BS.drop 1 suffix), s3)

-- ============================================================================
-- Helpers
-- ============================================================================

assertEqual :: (Eq a, Show a) => String -> a -> a -> IO ()
assertEqual context expected actual
  | expected == actual = return ()
  | otherwise = failTest context ("expected " ++ show expected ++ ", got " ++ show actual)

failTest :: String -> String -> IO a
failTest name reason = do
  hPutStrLn stderr ("FAILED [" ++ name ++ "]: " ++ reason)
  ioError (userError ("Test " ++ name ++ " failed: " ++ reason))
