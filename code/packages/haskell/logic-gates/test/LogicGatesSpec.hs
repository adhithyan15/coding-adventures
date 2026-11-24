module LogicGatesSpec (spec) where

import Test.Hspec
import LogicGates

spec :: Spec
spec = do
    basicSpec
    hardeningSpec

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

unaryTables :: [(String, Bit -> Either LogicGateError Bit, [(Bit, Bit)])]
unaryTables =
    [ ("notGate", notGate, [(0, 1), (1, 0)])
    , ("nandNot", nandNot, [(0, 1), (1, 0)])
    ]

binaryTables :: [(String, Bit -> Bit -> Either LogicGateError Bit, [(Bit, Bit, Bit)])]
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

-- Every Int in [-1024, 1024] except 0 and 1, plus the extremes next to the
-- Int boundaries.
sweepBits :: [Bit]
sweepBits =
    [x | x <- [-1024 .. 1024], x /= 0, x /= 1]
    ++ [minBound, minBound + 1, maxBound - 1, maxBound]

bitPairs :: [(Bit, Bit)]
bitPairs = [(a, b) | a <- [0, 1], b <- [0, 1]]

isRight' :: Either LogicGateError Bit -> Bool
isRight' (Right _) = True
isRight' _ = False

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

    describe "rejection of non-bits (structural InvalidBit)" $ do
        mapM_ (\(name, f, _) ->
            it (name ++ " reports InvalidBit \"a\" for every bad value") $
                mapM_ (\bad -> f bad `shouldBe` Left (InvalidBit "a" bad)) sweepBits) unaryTables
        mapM_ (\(name, f, _) -> do
            it (name ++ " reports InvalidBit \"a\" for a bad first argument") $
                mapM_ (\bad -> mapM_ (\ok -> f bad ok `shouldBe` Left (InvalidBit "a" bad)) [0, 1]) sweepBits
            it (name ++ " reports InvalidBit \"b\" for a bad second argument") $
                mapM_ (\bad -> mapM_ (\ok -> f ok bad `shouldBe` Left (InvalidBit "b" bad)) [0, 1]) sweepBits
            it (name ++ " reports the FIRST invalid parameter when both are bad") $
                mapM_ (\x -> mapM_ (\y -> f x y `shouldBe` Left (InvalidBit "a" x)) badBits) sweepBits) binaryTables

    describe "NAND-built gates never leak an inner parameter name" $ do
        it "nandOr 1 2 and nandOr 2 1 name a and b" $ do
            nandOr 1 2 `shouldBe` Left (InvalidBit "b" 2)
            nandOr 2 1 `shouldBe` Left (InvalidBit "a" 2)
        it "every NAND-built and delegating gate reports only a or b" $ do
            let names r = case r of
                    Left (InvalidBit n _) -> n `elem` ["a", "b"]
                    _ -> False
            mapM_ (\f -> mapM_ (\x -> names (f x 1) `shouldBe` True) badBits)
                [nandGate, norGate, xnorGate, nandAnd, nandOr, nandXor]
            mapM_ (\x -> names (nandNot x) `shouldBe` True) badBits

    describe "sequential and combinational errors are structural" $ do
        it "dLatch names its own parameters" $ do
            dLatch 2 1 0 1 `shouldBe` Left (InvalidBit "d" 2)
            dLatch 1 3 0 1 `shouldBe` Left (InvalidBit "e" 3)
        it "dFlipFlop names its own parameters" $ do
            dFlipFlop 1 5 0 1 0 1 `shouldBe` Left (InvalidBit "clock" 5)
        it "mux2 names its own parameters" $
            mux2 0 1 7 `shouldBe` Left (InvalidBit "sel" 7)
        it "shape errors are InvalidShape, not InvalidBit" $
            mux8 [0, 1] [0, 0, 0] `shouldBe` Left (InvalidShape "inputs must have length 8")

    describe "valid input never fails" $ do
        it "no unary gate returns Left on 0 or 1" $
            mapM_ (\(_, f, _) -> mapM_ (\a -> isRight' (f a) `shouldBe` True) [0, 1]) unaryTables
        it "no binary gate returns Left on any pair of bits" $
            mapM_ (\(_, f, _) -> mapM_ (\(a, b) -> isRight' (f a b) `shouldBe` True) bitPairs) binaryTables

-- ---------------------------------------------------------------------------
-- Totality and bounds (security review follow-up)
-- ---------------------------------------------------------------------------

isShape :: Either LogicGateError a -> Bool
isShape (Left (InvalidShape _)) = True
isShape _ = False

hardeningSpec :: Spec
hardeningSpec = describe "Totality and bounds" $ do
    describe "muxN" $ do
        it "selects correctly for valid shapes" $ do
            muxN [0, 1] [1] `shouldBe` Right 1
            muxN [0, 1, 0, 0] [1, 0] `shouldBe` Right 1
            muxN [0, 0, 0, 0, 0, 0, 0, 1] [1, 1, 1] `shouldBe` Right 1
            muxN [0, 0, 1, 0] [0, 1] `shouldBe` Right 1
        it "rejects empty sel without throwing" $
            isShape (muxN [0, 1, 0, 1] []) `shouldBe` True
        it "rejects short sel" $
            isShape (muxN [0, 1, 0, 1, 0, 1, 0, 1] [1]) `shouldBe` True
        it "rejects oversized sel" $
            isShape (muxN [0, 1] [0, 0]) `shouldBe` True
        it "rejects non-power-of-two and tiny inputs" $ do
            isShape (muxN [0, 1, 0] [0, 0]) `shouldBe` True
            isShape (muxN [1] [0]) `shouldBe` True
            isShape (muxN [] [0]) `shouldBe` True
        it "rejects sel wider than 16 bits before sizing" $
            isShape (muxN [0, 1] (replicate 64 0)) `shouldBe` True
    describe "decoder / demux select cap" $ do
        it "accepts 16 bits" $ do
            fmap length (decoder (replicate 16 0)) `shouldBe` Right 65536
            fmap length (demux 1 (replicate 16 0)) `shouldBe` Right 65536
        it "rejects 17, 63, 64 and 65 bits" $
            mapM_ (\n -> do
                isShape (decoder (replicate n 0)) `shouldBe` True
                isShape (demux 1 (replicate n 0)) `shouldBe` True) [17, 63, 64, 65]
    describe "sequential width caps and strictness" $ do
        it "register rejects qBits / masterQBits length mismatch" $ do
            isShape (register [1, 0] 1 [] []) `shouldBe` True
            isShape (register [1, 0] 1 [0, 0] [0]) `shouldBe` True
        it "register accepts matching lengths" $
            fmap fst (register [1, 0] 1 [0, 0] [0, 0]) `shouldBe` Right [1, 0]
        it "register rejects widths over maxDataWidth" $
            isShape (register (replicate (maxDataWidth + 1) 0) 0 [] []) `shouldBe` True
        it "shiftRegister rejects widths over maxDataWidth" $
            isShape (shiftRegister 0 0 (replicate (maxDataWidth + 1) 0)) `shouldBe` True
        it "counter rejects widths over maxCounterWidth, accepts the cap" $ do
            isShape (counter 1 0 (replicate (maxCounterWidth + 1) 0)) `shouldBe` True
            fmap (\(q, _, _) -> length q) (counter 1 0 (replicate maxCounterWidth 0)) `shouldBe` Right maxCounterWidth
        it "shift register shifts" $ do
            fmap (\(q, o, _) -> (q, o)) (shiftRegister 1 1 [0, 0, 0, 0]) `shouldBe` Right ([1, 0, 0, 0], 0)
