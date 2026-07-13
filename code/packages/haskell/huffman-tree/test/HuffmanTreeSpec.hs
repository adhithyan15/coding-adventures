module HuffmanTreeSpec (spec) where

import Data.Either (isLeft)
import Data.List (isPrefixOf)
import qualified Data.Map.Strict as Map
import HuffmanTree
import Test.Hspec

spec :: Spec
spec = do
    describe "build" $ do
        it "constructs single-, two-, and three-symbol trees" $ do
            fmap symbolCount (build [pair 65 5]) `shouldBe` Right 1
            fmap weight (build [pair 65 3, pair 66 1]) `shouldBe` Right 4
            fmap weight classicTree `shouldBe` Right 6
        it "rejects an empty alphabet" $
            build [] `shouldSatisfy` isLeft
        it "rejects zero and negative frequencies with useful errors" $ do
            build [pair 65 0] `shouldBe`
                Left "frequency must be positive; got symbol=65, freq=0"
            build [pair 42 (-5)] `shouldBe`
                Left "frequency must be positive; got symbol=42, freq=-5"
        it "builds a valid 256-symbol alphabet" $ do
            tree <- expectTree (build [pair value (value + 1) | value <- [0 .. 255]])
            symbolCount tree `shouldBe` 256
            isValid tree `shouldBe` True

    describe "codeTable and codeFor" $ do
        it "assigns the shortest code to the highest-frequency symbol" $ do
            tree <- expectTree classicTree
            let table = codeTable tree
            Map.lookup 65 table `shouldBe` Just "0"
            Map.lookup 67 table `shouldBe` Just "10"
            Map.lookup 66 table `shouldBe` Just "11"
        it "uses zero for a single-symbol tree" $ do
            tree <- expectTree (build [pair 65 1])
            codeTable tree `shouldBe` Map.singleton 65 "0"
            codeFor tree 65 `shouldBe` Just "0"
        it "returns Nothing for a missing symbol" $ do
            tree <- expectTree (build [pair 65 3, pair 66 1])
            codeFor tree 99 `shouldBe` Nothing
        it "produces prefix-free codes" $ do
            tree <- expectTree (build [pair value (value + 1) | value <- [0 .. 9]])
            prefixFree (Map.elems (codeTable tree)) `shouldBe` True

    describe "canonicalCodeTable" $ do
        it "assigns the canonical AAABBC table" $ do
            tree <- expectTree classicTree
            canonicalCodeTable tree `shouldBe`
                Map.fromList [(65, "0"), (66, "10"), (67, "11")]
        it "preserves every ordinary code length" $ do
            tree <- expectTree (build [pair value (value + 1) | value <- [0 .. 7]])
            let ordinary = Map.map length (codeTable tree)
                canonical = Map.map length (canonicalCodeTable tree)
            canonical `shouldBe` ordinary
        it "uses zero for one symbol and remains prefix-free" $ do
            single <- expectTree (build [pair 65 5])
            canonicalCodeTable single `shouldBe` Map.singleton 65 "0"
            tree <- expectTree (build [pair value (value + 1) | value <- [0 .. 9]])
            prefixFree (Map.elems (canonicalCodeTable tree)) `shouldBe` True

    describe "decodeAll" $ do
        it "round-trips the AAABBC message" $ do
            tree <- expectTree classicTree
            let symbols = [65, 65, 65, 66, 66, 67]
                bits = encode (codeTable tree) symbols
            decodeAll tree bits (length symbols) `shouldBe` Right symbols
        it "round-trips a single-symbol message" $ do
            tree <- expectTree (build [pair 65 5])
            decodeAll tree "000" 3 `shouldBe` Right [65, 65, 65]
        it "round-trips all byte values" $ do
            let symbols = [0 .. 255]
            tree <- expectTree (build [pair value (value + 1) | value <- symbols])
            decodeAll tree (encode (codeTable tree) symbols) 256 `shouldBe` Right symbols
        it "reports an exhausted stream" $ do
            tree <- expectTree classicTree
            decodeAll tree "0" 5 `shouldBe`
                Left "Bit stream exhausted after 1 symbols; expected 5"
        it "decodes zero requested symbols without consuming input" $ do
            tree <- expectTree classicTree
            decodeAll tree "anything" 0 `shouldBe` Right []

    describe "inspection" $ do
        it "reports weight, depth, and symbol count" $ do
            tree <- expectTree classicTree
            weight tree `shouldBe` 6
            depth tree `shouldBe` 2
            symbolCount tree `shouldBe` 3
        it "reports depth zero for a single leaf and one for two leaves" $ do
            single <- expectTree (build [pair 65 1])
            two <- expectTree (build [pair 65 3, pair 66 1])
            depth single `shouldBe` 0
            depth two `shouldBe` 1
        it "returns leaves from left to right with matching codes" $ do
            tree <- expectTree classicTree
            leaves tree `shouldBe` [(65, "0"), (67, "10"), (66, "11")]

    describe "deterministic tie-breaking and invariants" $ do
        it "places the lower symbol on the left for equal-weight leaves" $ do
            tree <- expectTree (build [pair 65 1, pair 66 1])
            codeTable tree `shouldBe` Map.fromList [(65, "0"), (66, "1")]
        it "places an equal-weight leaf before an internal node" $ do
            tree <- expectTree (build [pair 65 1, pair 66 1, pair 67 2])
            codeTable tree `shouldBe`
                Map.fromList [(65, "10"), (66, "11"), (67, "0")]
        it "uses FIFO creation order for equal-weight internal nodes" $ do
            tree <- expectTree (build [pair 65 1, pair 66 1, pair 67 1, pair 68 1])
            codeTable tree `shouldBe`
                Map.fromList [(65, "00"), (66, "01"), (67, "10"), (68, "11")]
        it "builds identical trees for identical equal-weight inputs" $ do
            first <- expectTree (build [pair value 1 | value <- [0 .. 7]])
            second <- expectTree (build [pair value 1 | value <- [0 .. 7]])
            codeTable first `shouldBe` codeTable second
            isValid first `shouldBe` True
        it "detects duplicate symbols" $ do
            tree <- expectTree (build [pair 7 1, pair 7 2])
            isValid tree `shouldBe` False
  where
    classicTree = build [pair 65 3, pair 66 2, pair 67 1]

pair :: Int -> Int -> WeightPair
pair = WeightPair

expectTree :: Either String HuffmanTree -> IO HuffmanTree
expectTree result =
    case result of
        Left message -> expectationFailure message >> fail message
        Right tree -> pure tree

encode :: Map.Map Int String -> [Int] -> String
encode table = concatMap (table Map.!)

prefixFree :: [String] -> Bool
prefixFree codes =
    and
        [ not (first `isPrefixOf` second)
        | (firstIndex, first) <- zip [0 :: Int ..] codes
        , (secondIndex, second) <- zip [0 :: Int ..] codes
        , firstIndex /= secondIndex
        ]
