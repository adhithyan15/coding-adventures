{-# LANGUAGE ScopedTypeVariables #-}
-- | LGE-HS-HARDEN: bounded list snapshots, empty-state rejection, forced
-- results, and the full rejection matrix.
module HardenSpec (spec) where

import Control.Exception (SomeException, evaluate, try)
import System.Timeout (timeout)
import Test.Hspec
import LogicGates

-- Values that are not bits, including every Int width boundary.
wideInts :: [Int]
wideInts =
    [ 2 ^ (31 :: Int), 2 ^ (32 :: Int), 2 ^ (32 :: Int) + 1
    , 2 ^ (53 :: Int) + 1, maxBound, minBound, -1, 2 ]

unary :: [(String, Int -> Either LogicGateError Int)]
unary = [("notGate", notGate), ("nandNot", nandNot)]

binary :: [(String, Int -> Int -> Either LogicGateError Int)]
binary =
    [ ("andGate", andGate), ("orGate", orGate), ("xorGate", xorGate)
    , ("nandGate", nandGate), ("norGate", norGate), ("xnorGate", xnorGate)
    , ("nandAnd", nandAnd), ("nandOr", nandOr), ("nandXor", nandXor) ]

-- | Run a pure result under a 5 second guard, forcing it completely via
-- 'show'.  Nothing = timed out; Just (Left e) = threw; Just (Right s) = value.
guarded :: Show a => a -> IO (Maybe (Either String String))
guarded x = timeout 5000000 $ do
    r <- try (evaluate (length (show x) `seq` show x))
    return $ case r of
        Left (e :: SomeException) -> Left (show e)
        Right s -> Right s

-- | The function must itself return Left (InvalidShape _) promptly.
rejectsPromptly :: Show a => Either LogicGateError a -> Expectation
rejectsPromptly r = do
    out <- guarded r
    case out of
        Nothing -> expectationFailure "timed out"
        Just (Left e) -> expectationFailure ("threw: " ++ e)
        Just (Right _) -> case r of
            Left (InvalidShape _) -> return ()
            _ -> expectationFailure "expected Left InvalidShape"

neverThrows :: Show a => a -> Expectation
neverThrows x = do
    out <- guarded x
    case out of
        Just (Right _) -> return ()
        Just (Left e) -> expectationFailure ("threw: " ++ e)
        Nothing -> expectationFailure "timed out"

isLeftNoThrow :: Show a => Either LogicGateError a -> Expectation
isLeftNoThrow r = do
    out <- guarded r
    case (out, r) of
        (Just (Right _), Left _) -> return ()
        (Just (Left e), _) -> expectationFailure ("threw: " ++ e)
        _ -> expectationFailure "expected Left"

inf :: [Int]
inf = cycle [0]

spec :: Spec
spec = describe "LGE-HS-HARDEN" $ do
    describe "wide integers on all 11 gates, both positions" $ do
        mapM_ (\(n, f) -> it (n ++ " rejects wide ints") $
            mapM_ (\v -> f v `shouldBe` Left (InvalidBit "a" v)) wideInts) unary
        mapM_ (\(n, f) -> it (n ++ " rejects wide ints in a and b") $
            mapM_ (\v -> do
                f v 1 `shouldBe` Left (InvalidBit "a" v)
                f 1 v `shouldBe` Left (InvalidBit "b" v)
                -- a decisive first argument must not short-circuit validation
                f 0 v `shouldBe` Left (InvalidBit "b" v)
                f v 0 `shouldBe` Left (InvalidBit "a" v)) wideInts) binary

    describe "bad-bit rows" $ do
        it "demux" $ do
            demux 2 [0] `shouldBe` Left (InvalidBit "dataBit" 2)
            demux 1 [0, 3] `shouldBe` Left (InvalidBit "sel" 3)
        it "decoder" $ decoder [0, 2] `shouldBe` Left (InvalidBit "inputs" 2)
        it "encoder" $ encoder [0, 5] `shouldBe` Left (InvalidBit "inputs" 5)
        it "priorityEncoder" $ priorityEncoder [-1, 0] `shouldBe` Left (InvalidBit "inputs" (-1))
        it "triState" $ do
            triState 2 1 `shouldBe` Left (InvalidBit "dataBit" 2)
            triState 0 maxBound `shouldBe` Left (InvalidBit "enable" maxBound)
            triState 1 1 `shouldBe` Right (Just 1)
            triState 1 0 `shouldBe` Right Nothing
        it "mux4" $ do
            mux4 0 0 0 9 [0, 0] `shouldBe` Left (InvalidBit "d3" 9)
            mux4 0 0 0 0 [0, 7] `shouldBe` Left (InvalidBit "sel" 7)
            mux4 0 0 0 0 [7] `shouldBe` Left (InvalidShape "sel must have length 2")
        it "mux8" $ do
            mux8 [0, 0, 0, 0, 0, 0, 0, 4] [0, 0, 0] `shouldBe` Left (InvalidBit "inputs" 4)
            mux8 (replicate 8 0) [0, 0, 6] `shouldBe` Left (InvalidBit "sel" 6)

    describe "infinite lists are rejected by the function itself" $ do
        it "mux4 sel" $ rejectsPromptly (mux4 0 0 0 0 inf)
        it "mux8 inputs" $ rejectsPromptly (mux8 inf [0, 0, 0])
        it "mux8 sel" $ rejectsPromptly (mux8 (replicate 8 0) inf)
        it "muxN inputs" $ rejectsPromptly (muxN inf [0])
        it "muxN sel" $ rejectsPromptly (muxN [0, 1] inf)
        it "demux" $ rejectsPromptly (demux 1 inf)
        it "decoder" $ rejectsPromptly (decoder inf)
        it "encoder" $ rejectsPromptly (encoder inf)
        it "priorityEncoder" $ rejectsPromptly (priorityEncoder inf)
        it "register dataBits" $ rejectsPromptly (register inf 0 [] [])
        it "register qBits" $ rejectsPromptly (register [0] 0 inf [0])
        it "register masterQBits" $ rejectsPromptly (register [0] 0 [0] inf)
        it "shiftRegister" $ rejectsPromptly (shiftRegister 0 0 inf)
        it "counter" $ rejectsPromptly (counter 0 0 inf)
        it "huge finite lists (cap + 1)" $ do
            rejectsPromptly (encoder (replicate (maxInputWidth + 1) 0))
            rejectsPromptly (priorityEncoder (replicate (maxInputWidth + 1) 0))
            rejectsPromptly (muxN (replicate (maxInputWidth + 1) 0) [0])

    describe "empty state is rejected, not defaulted" $ do
        it "shiftRegister" $ shiftRegister 1 1 [] `shouldBe` Left (InvalidShape "qBits must not be empty")
        it "counter" $ counter 1 0 [] `shouldBe` Left (InvalidShape "qBits must not be empty")

    describe "results are fully evaluated" $ do
        it "valid inputs never throw when forced" $ do
            neverThrows (decoder [1, 0, 1])
            neverThrows (demux 1 [1, 1])
            neverThrows (encoder [0, 0, 1, 0])
            neverThrows (priorityEncoder [1, 0, 1, 0])
            neverThrows (priorityEncoder [0, 0, 0, 0])
            neverThrows (muxN [0, 1, 1, 0] [1, 0])
            neverThrows (mux4 0 1 0 1 [1, 0])
            neverThrows (mux8 [0, 1, 0, 1, 0, 1, 0, 1] [1, 1, 0])
            neverThrows (register [1, 0, 1] 1 [0, 0, 0] [0, 0, 0])
            neverThrows (shiftRegister 1 1 [0, 1, 0, 1])
            neverThrows (counter 1 0 [1, 1, 1, 1])
            neverThrows (counter 1 1 [1, 1, 1, 1])
            neverThrows (srLatch 1 0 0 1)
            neverThrows (dFlipFlop 1 1 0 1 0 1)
        it "invalid inputs return Left without throwing" $ do
            isLeftNoThrow (decoder [2])
            isLeftNoThrow (shiftRegister 0 0 [])
            isLeftNoThrow (demux 1 (replicate 17 0))
            isLeftNoThrow (mux8 [] [])
            isLeftNoThrow (counter 0 0 [3])

    describe "caller contract and bounded names" $ do
        it "a bottom element supplied by the caller is the caller's own bottom" $
            evaluate (muxN [0, undefined] [0]) `shouldThrow` anyException
        it "validateBits rejects infinite and over-cap lists" $ do
            rejectsPromptly (validateBits "x" inf)
            rejectsPromptly (validateBits "x" (replicate 65537 0))
            validateBits "x" (replicate 65536 0) `shouldBe` Right (replicate 65536 0)
        it "validateBit bounds the name" $ do
            validateBit (replicate 100 'n') 2 `shouldBe` Left (InvalidBit (replicate 64 'n') 2)
            validateBit "a\nb\ESC" 2 `shouldBe` Left (InvalidBit "a?b?" 2)

    describe "valid paths" $ do
        it "encoder" $ do
            encoder [0, 0, 1, 0] `shouldBe` Right [0, 1]
            encoder [0, 1, 0, 0] `shouldBe` Right [1, 0]
            encoder [1, 1, 0, 0] `shouldBe` Left (InvalidShape "exactly one input must be 1")
        it "priorityEncoder" $ do
            priorityEncoder [1, 0, 1, 0] `shouldBe` Right ([0, 1], 1)
            priorityEncoder [1, 1, 0, 0] `shouldBe` Right ([1, 0], 1)
            priorityEncoder [0, 0, 0, 0] `shouldBe` Right ([0, 0], 0)
        it "triState" $ do
            triState 0 1 `shouldBe` Right (Just 0)
            triState 0 0 `shouldBe` Right Nothing
        it "mux4" $ do
            mux4 1 0 0 0 [0, 0] `shouldBe` Right 1
            mux4 0 1 0 0 [1, 0] `shouldBe` Right 1
            mux4 0 0 1 0 [0, 1] `shouldBe` Right 1
            mux4 0 0 0 1 [1, 1] `shouldBe` Right 1
            mux4 1 1 1 0 [1, 1] `shouldBe` Right 0
        it "srLatch" $ do
            srLatch 1 0 0 1 `shouldBe` Right (LatchState 1 0)
            srLatch 0 1 1 0 `shouldBe` Right (LatchState 0 1)
            srLatch 0 0 1 0 `shouldBe` Right (LatchState 1 0)
