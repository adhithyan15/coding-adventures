module BrainfuckSpec (spec) where

import Brainfuck
import Test.Hspec

spec :: Spec
spec = describe "Brainfuck" $ do
    it "filters comments during tokenization" $ do
        tokenize "++ hello >." `shouldBe` "++>."

    it "parses nested loops" $ do
        parseSource "+[->+<]" `shouldBe` Right [Increment, Loop [Decrement, MoveRight, Increment, MoveLeft]]

    it "rejects unbalanced loops" $ do
        parseSource "+[" `shouldSatisfy` either (const True) (const False)
