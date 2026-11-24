module LogicGatesSpec (spec) where

import Data.List (isInfixOf)
import Test.Hspec
import LogicGates

spec :: Spec
spec = do
    basicSpec

    describe "Combinational Logic" $ do
        it "multiplexes correctly" $ do
            mux2 0 1 1 `shouldBe` Right 1
            mux2 1 0 0 `shouldBe` Right 1

        it "decodes correctly" $ do
            decoder [1, 0] `shouldBe` Right [0, 1, 0, 0]

    describe "Sequential Logic" $ do
        it "latches correctly" $ do
            let dl = dLatch 1 1 0 1
            fmap qOut dl `shouldBe` Right 1

        it "flipflops correctly" $ do
            let ff = dFlipFlop 1 1 0 1 1 0
            fmap ffQ ff `shouldBe` Right 1

-- ---------------------------------------------------------------------------
-- Basic gates
-- ---------------------------------------------------------------------------

-- Every expected value below is a literal written from the truth tables, never
-- computed by calling the code under test.

unaryTables :: [(String, Bit -> Either String Bit, [(Bit, Bit)])]
unaryTables =
    [ ("notGate", notGate, [(0, 1), (1, 0)])
    , ("nandNot", nandNot, [(0, 1), (1, 0)])
    ]

binaryTables :: [(String, Bit -> Bit -> Either String Bit, [(Bit, Bit, Bit)])]
binaryTables =
    [ ("andGate",  andGate,  [(0,0,0),(0,1,0),(1,0,0),(1,1,1)])
    , ("orGate",   orGate,   [(0,0,0),(0,1,1),(1,0,1),(1,1,1)])
    , ("xorGate",  xorGate,  [(0,0,0),(0,1,1),(1,0,1),(1,1,0)])
    , ("nandGate", nandGate, [(0,0,1),(0,1,1),(1,0,1),(1,1,0)])
    , ("norGate",  norGate,  [(0,0,1),(0,1,0),(1,0,0),(1,1,0)])
    , ("xnorGate", xnorGate, [(0,0,1),(0,1,0),(1,0,0),(1,1,1)])
    , ("nandAnd",  nandAnd,  [(0,0,0),(0,1,0),(1,0,0),(1,1,1)])
    , ("nandOr",   nandOr,   [(0,0,0),(0,1,1),(1,0,1),(1,1,1)])
    , ("nandXor",  nandXor,  [(0,0,0),(0,1,1),(1,0,1),(1,1,0)])
    ]

badBits :: [Bit]
badBits = [2, -1, 255, 256, minBound, maxBound]

bitPairs :: [(Bit, Bit)]
bitPairs = [(a, b) | a <- [0, 1], b <- [0, 1]]

isRight' :: Either String Bit -> Bool
isRight' (Right _) = True
isRight' _ = False

-- A Left whose message mentions the offending value.
rejects :: Bit -> Either String Bit -> Expectation
rejects bad r = case r of
    Left msg -> msg `shouldSatisfy` (show bad `isInfixOf`)
    Right v  -> expectationFailure ("expected Left for " ++ show bad ++ ", got Right " ++ show v)

basicSpec :: Spec
basicSpec = describe "Basic logic gates" $ do
    describe "truth tables" $ do
        mapM_ (\(name, f, rows) ->
            it (name ++ " matches its truth table") $
                mapM_ (\(a, out) -> f a `shouldBe` Right out) rows) unaryTables
        mapM_ (\(name, f, rows) ->
            it (name ++ " matches its truth table") $
                mapM_ (\(a, b, out) -> f a b `shouldBe` Right out) rows) binaryTables

    describe "NAND-built gates equal their direct counterparts" $ do
        it "nandNot == notGate" $
            mapM_ (\a -> nandNot a `shouldBe` notGate a) [0, 1]
        it "nandAnd == andGate" $
            mapM_ (\(a, b) -> nandAnd a b `shouldBe` andGate a b) bitPairs
        it "nandOr == orGate" $
            mapM_ (\(a, b) -> nandOr a b `shouldBe` orGate a b) bitPairs
        it "nandXor == xorGate" $
            mapM_ (\(a, b) -> nandXor a b `shouldBe` xorGate a b) bitPairs

    describe "rejection of non-bits" $ do
        mapM_ (\(name, f, _) ->
            it (name ++ " rejects bad input") $
                mapM_ (\bad -> rejects bad (f bad)) badBits) unaryTables
        mapM_ (\(name, f, _) -> do
            it (name ++ " rejects a bad first argument") $
                mapM_ (\bad -> mapM_ (\ok -> rejects bad (f bad ok)) [0, 1]) badBits
            it (name ++ " rejects a bad second argument") $
                mapM_ (\bad -> mapM_ (\ok -> rejects bad (f ok bad)) [0, 1]) badBits) binaryTables

    describe "valid input never fails" $ do
        it "no unary gate returns Left on 0 or 1" $
            mapM_ (\(_, f, _) -> mapM_ (\a -> isRight' (f a) `shouldBe` True) [0, 1]) unaryTables
        it "no binary gate returns Left on any pair of bits" $
            mapM_ (\(_, f, _) -> mapM_ (\(a, b) -> isRight' (f a b) `shouldBe` True) bitPairs) binaryTables
