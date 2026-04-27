module StateMachineSpec (spec) where

import Test.Hspec

import StateMachine

spec :: Spec
spec = do
    describe "DFA" $ do
        it "processes a turnstile sequence" $ do
            let machine =
                    newDFA
                        ["locked", "unlocked"]
                        ["coin", "push"]
                        [ (("locked", "coin"), "unlocked")
                        , (("locked", "push"), "locked")
                        , (("unlocked", "coin"), "unlocked")
                        , (("unlocked", "push"), "locked")
                        ]
                        "locked"
                        ["unlocked"]
            fmap dfaCurrentState (processDFASequence ["coin", "push"] =<< machine) `shouldBe` Right "locked"
            fmap (acceptsDFA ["coin"]) machine `shouldBe` Right (Right True)

        it "reports missing transitions in validation" $ do
            let machine =
                    newDFA
                        ["q0", "q1"]
                        ["a", "b"]
                        [(("q0", "a"), "q1")]
                        "q0"
                        ["q1"]
            fmap validateDFA machine `shouldBe` Right ["missing transition for (q0, b)", "missing transition for (q1, a)", "missing transition for (q1, b)"]

    describe "NFA" $ do
        it "accepts through epsilon transitions and converts to a DFA" $ do
            let machine =
                    newNFA
                        ["q0", "q1", "q2"]
                        ["a", "b"]
                        [ (("q0", Nothing), ["q1"])
                        , (("q1", Just "a"), ["q1"])
                        , (("q1", Just "b"), ["q2"])
                        ]
                        "q0"
                        ["q2"]
            fmap (acceptsNFA ["a", "a", "b"]) machine `shouldBe` Right (Right True)
            fmap (\nfa -> nfaToDFA nfa >>= acceptsDFA ["a", "b"]) machine `shouldBe` Right (Right True)

    describe "minimizeDFA" $ do
        it "merges equivalent DFA states" $ do
            let machine =
                    newDFA
                        ["A", "B", "C"]
                        ["0", "1"]
                        [ (("A", "0"), "B")
                        , (("A", "1"), "C")
                        , (("B", "0"), "B")
                        , (("B", "1"), "C")
                        , (("C", "0"), "B")
                        , (("C", "1"), "C")
                        ]
                        "A"
                        ["C"]
            fmap (length . dfaStatesSet) (minimizeDFA =<< machine) `shouldBe` Right 2

    describe "PushdownAutomaton" $ do
        it "recognizes balanced parentheses" $ do
            let open = "("
                close = ")"
                machine =
                    newPushdownAutomaton
                        ["q0", "accept"]
                        [open, close]
                        ["$", open]
                        [ PDATransition "q0" (Just open) "$" "q0" ["$", open]
                        , PDATransition "q0" (Just open) open "q0" [open, open]
                        , PDATransition "q0" (Just close) open "q0" []
                        , PDATransition "q0" Nothing "$" "accept" []
                        ]
                        "q0"
                        "$"
                        ["accept"]
            fmap (acceptsPDA [open, open, close, close]) machine `shouldBe` Right (Right True)

    describe "ModalStateMachine" $ do
        it "switches modes and resets the target DFA" $ do
            let dataMode =
                    newDFA
                        ["idle", "seen"]
                        ["char"]
                        [ (("idle", "char"), "seen")
                        , (("seen", "char"), "seen")
                        ]
                        "idle"
                        ["seen"]
                tagMode =
                    newDFA
                        ["tag-start", "tag-seen"]
                        ["char"]
                        [ (("tag-start", "char"), "tag-seen")
                        , (("tag-seen", "char"), "tag-seen")
                        ]
                        "tag-start"
                        ["tag-seen"]
                modal =
                    newModalStateMachine
                        [("data", either error id dataMode), ("tag", either error id tagMode)]
                        [(("data", "enter-tag"), "tag"), (("tag", "exit-tag"), "data")]
                        "data"
            fmap modalCurrentMode (switchMode "enter-tag" =<< modal) `shouldBe` Right "tag"
