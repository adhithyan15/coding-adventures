-- | Unit tests for the ReedSolomon module.
--
-- Tests: generator polynomial construction, encoding, syndrome computation,
-- round-trip correctness, error correction up to capacity, and failure
-- beyond capacity.
module ReedSolomonSpec (spec) where

import Test.Hspec
import Data.Bits  (xor)
import Data.Array ((!))
import ReedSolomon
import GF256      (gfPow, expTable, logTable)

spec :: Spec
spec = do
    -- -----------------------------------------------------------------------
    -- buildGenerator
    -- -----------------------------------------------------------------------
    describe "buildGenerator" $ do
        it "InvalidInput for nCheck = 0" $
            buildGenerator 0 `shouldSatisfy` isLeft

        it "InvalidInput for nCheck = 1 (odd)" $
            buildGenerator 1 `shouldSatisfy` isLeft

        it "InvalidInput for nCheck = 3 (odd)" $
            buildGenerator 3 `shouldSatisfy` isLeft

        it "nCheck=2: g = [8, 6, 1]  (i.e. (x+2)(x+4))" $
            buildGenerator 2 `shouldBe` Right [8, 6, 1]

        it "nCheck=2: length = nCheck+1 = 3" $
            fmap length (buildGenerator 2) `shouldBe` Right 3

        it "nCheck=4: monic (last coefficient = 1)" $
            fmap last (buildGenerator 4) `shouldBe` Right 1

        it "nCheck=4: length = 5" $
            fmap length (buildGenerator 4) `shouldBe` Right 5

        it "roots: g(alpha^i) = 0 for i = 1..nCheck" $ do
            case buildGenerator 4 of
              Left _  -> expectationFailure "buildGenerator 4 failed"
              Right g ->
                mapM_ (\i ->
                    hornerLE g (gfPow 2 i) `shouldBe` 0
                  ) [1..4]

    -- -----------------------------------------------------------------------
    -- encode
    -- -----------------------------------------------------------------------
    describe "encode" $ do
        it "InvalidInput for nCheck = 0" $
            encode [1, 2, 3] 0 `shouldSatisfy` isLeft

        it "InvalidInput for nCheck = 3 (odd)" $
            encode [1, 2, 3] 3 `shouldSatisfy` isLeft

        it "output length = message length + nCheck" $
            fmap length (encode [4, 3, 2, 1] 2) `shouldBe` Right 6

        it "codeword is systematic: first k bytes equal message" $ do
            let msg = [4, 3, 2, 1]
            case encode msg 2 of
              Left _  -> expectationFailure "encode failed"
              Right cw -> take (length msg) cw `shouldBe` msg

        it "nCheck=2: syndromes of encoded codeword are all zero" $ do
            let msg = [4, 3, 2, 1]
            case encode msg 2 of
              Left _  -> expectationFailure "encode failed"
              Right cw -> syndromes cw 2 `shouldBe` replicate 2 0

        it "nCheck=4: syndromes all zero" $ do
            let msg = [1, 2, 3, 4, 5]
            case encode msg 4 of
              Left _  -> expectationFailure "encode failed"
              Right cw -> syndromes cw 4 `shouldBe` replicate 4 0

        it "nCheck=8: syndromes all zero" $ do
            let msg = [10, 20, 30, 40, 50, 60, 70, 80]
            case encode msg 8 of
              Left _  -> expectationFailure "encode failed"
              Right cw -> syndromes cw 8 `shouldBe` replicate 8 0

    -- -----------------------------------------------------------------------
    -- syndromes
    -- -----------------------------------------------------------------------
    describe "syndromes" $ do
        it "all zero for a valid codeword" $ do
            let msg = [1, 2, 3, 4, 5]
            case encode msg 4 of
              Left _ -> expectationFailure "encode failed"
              Right cw -> syndromes cw 4 `shouldBe` replicate 4 0

        it "not all zero after single-byte corruption" $ do
            let msg = [1, 2, 3, 4, 5]
            case encode msg 4 of
              Left _ -> expectationFailure "encode failed"
              Right cw ->
                all (== 0) (syndromes (corruptAt 0 0xFF cw) 4)
                    `shouldBe` False

    -- -----------------------------------------------------------------------
    -- decode — round-trip, no errors
    -- -----------------------------------------------------------------------
    describe "decode round-trip" $ do
        it "decode(encode(msg, 2), 2) = msg" $
            roundTrip [4, 3, 2, 1] 2

        it "decode(encode(msg, 4), 4) = msg" $
            roundTrip [1, 2, 3, 4, 5] 4

        it "decode with nCheck=8" $
            roundTrip [10, 20, 30, 40, 50, 60, 70, 80] 8

        it "decode ASCII 'Hello' with nCheck=4" $
            roundTrip [72, 101, 108, 108, 111] 4

        it "InvalidInput for nCheck = 0" $
            decode [1, 2, 3] 0 `shouldSatisfy` isLeft

        it "InvalidInput when received shorter than nCheck" $
            decode [1] 4 `shouldSatisfy` isLeft

    -- -----------------------------------------------------------------------
    -- decode — error correction up to capacity
    -- -----------------------------------------------------------------------
    describe "decode error correction" $ do
        it "corrects 1 error with nCheck=2 (t=1)" $
            correctErrors [1, 2, 3, 4] 2 [0] [0xFF]

        it "corrects last-byte error with nCheck=2" $ do
            let msg = [1, 2, 3, 4]
            case encode msg 2 of
              Left _ -> expectationFailure "encode failed"
              Right cw ->
                decode (corruptAt (length cw - 1) 0xAB cw) 2 `shouldBe` Right msg

        it "corrects 2 errors with nCheck=4 (t=2)" $
            correctErrors [1, 2, 3, 4, 5] 4 [1, 3] [0xAA, 0xBB]

        it "corrects 4 errors with nCheck=8 (t=4)" $
            correctErrors [10, 20, 30, 40, 50] 8 [0, 2, 4, 6] [0xFF, 0xAA, 0xBB, 0xCC]

        it "corrects error in the check-byte region" $ do
            let msg = [1, 2, 3, 4, 5]
            case encode msg 4 of
              Left _ -> expectationFailure "encode failed"
              Right cw ->
                decode (corruptAt (length msg) 0xAA cw) 4 `shouldBe` Right msg

    -- -----------------------------------------------------------------------
    -- decode — failure beyond capacity
    -- -----------------------------------------------------------------------
    describe "decode TooManyErrors" $ do
        it "TooManyErrors for t+1 errors with nCheck=2 (t=1)" $ do
            let msg = [1, 2, 3, 4]
            case encode msg 2 of
              Left _ -> expectationFailure "encode failed"
              Right cw ->
                decode (corruptAt 0 0xFF (corruptAt 1 0xAA cw)) 2
                    `shouldBe` Left TooManyErrors

        it "TooManyErrors for 3 errors with nCheck=4 (t=2)" $ do
            let msg = [1, 2, 3, 4, 5]
            case encode msg 4 of
              Left _ -> expectationFailure "encode failed"
              Right cw ->
                let corrupted = foldr (uncurry corruptAt) cw
                                      [(0, 0xFF), (1, 0xAA), (2, 0xBB)]
                in  decode corrupted 4 `shouldBe` Left TooManyErrors

    -- -----------------------------------------------------------------------
    -- errorLocator
    -- -----------------------------------------------------------------------
    describe "errorLocator" $ do
        it "zero syndromes → Λ = [1]" $
            errorLocator [0, 0, 0, 0] `shouldBe` [1]

        it "Λ[0] = 1 for any syndrome sequence" $ do
            let msg = [1, 2, 3, 4, 5]
            case encode msg 4 of
              Left _ -> expectationFailure "encode failed"
              Right cw ->
                let synds  = syndromes (corruptAt 0 0xFF cw) 4
                    lambda = errorLocator synds
                in  take 1 lambda `shouldBe` [1]

-- ---------------------------------------------------------------------------
-- Helpers
-- ---------------------------------------------------------------------------

-- | True when the value is a 'Left'.
isLeft :: Either a b -> Bool
isLeft (Left _) = True
isLeft _        = False

-- | Round-trip: encode then decode, verify recovery.
roundTrip :: [Int] -> Int -> Expectation
roundTrip msg nCheck =
    case encode msg nCheck of
      Left err -> expectationFailure ("encode failed: " ++ show err)
      Right cw -> decode cw nCheck `shouldBe` Right msg

-- | Encode, corrupt at given positions with given XOR masks, then decode.
correctErrors :: [Int] -> Int -> [Int] -> [Int] -> Expectation
correctErrors msg nCheck positions masks =
    case encode msg nCheck of
      Left err -> expectationFailure ("encode failed: " ++ show err)
      Right cw ->
        let corrupted = foldr (uncurry corruptAt) cw (zip positions masks)
        in  decode corrupted nCheck `shouldBe` Right msg

-- | XOR the byte at position @p@ with @mask@.
corruptAt :: Int -> Int -> [Int] -> [Int]
corruptAt p mask bytes =
    take p bytes ++ [(bytes !! p) `xor` mask] ++ drop (p + 1) bytes

-- | Evaluate a little-endian GF(256) polynomial at @x@ (Horner's method).
-- Used to verify generator roots in tests.
hornerLE :: [Int] -> Int -> Int
hornerLE coeffs x = foldr step 0 coeffs
  where
    step c acc = c `xor` gfMul x acc

    gfMul :: Int -> Int -> Int
    gfMul a b
        | a == 0 || b == 0 = 0
        | otherwise        = expTable ! ((logTable ! a + logTable ! b) `mod` 255)
