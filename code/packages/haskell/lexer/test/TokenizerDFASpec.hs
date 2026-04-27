module TokenizerDFASpec (spec) where

import Test.Hspec

import Lexer
import StateMachine.DFA

spec :: Spec
spec = do
    describe "classifyChar" $ do
        it "classifies structural characters" $ do
            classifyChar (Just 'a') `shouldBe` "alpha"
            classifyChar (Just '_') `shouldBe` "underscore"
            classifyChar (Just '\n') `shouldBe` "newline"
            classifyChar Nothing `shouldBe` "eof"

    describe "newTokenizerDFA" $ do
        it "dispatches digits to the number handler" $ do
            fmap dfaCurrentState (newTokenizerDFA >>= processDFA "digit") `shouldBe` Right "in_number"

        it "dispatches EOF to done" $ do
            fmap dfaCurrentState (newTokenizerDFA >>= processDFA "eof") `shouldBe` Right "done"
