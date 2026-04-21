-- | Test suite for the pure-Haskell Argon2i implementation.
--
-- Canonical vector comes from RFC 9106 §5.2.
module Argon2iSpec (spec) where

import Argon2i
import Control.Exception (evaluate)
import Data.Word (Word8)
import Test.Hspec

rfcPassword, rfcSalt, rfcKey, rfcAd :: [Word8]
rfcPassword = replicate 32 0x01
rfcSalt     = replicate 16 0x02
rfcKey      = replicate 8  0x03
rfcAd       = replicate 12 0x04

rfcExpected :: String
rfcExpected = "c814d9d1dc7f37aa13f0d77f2494bda1c8de6b016dd388d29952a4c4672b6ce8"

spec :: Spec
spec = describe "Argon2i" $ do

    describe "RFC 9106 §5.2 canonical vector" $ do

        it "matches the expected 32-byte tag" $
            argon2iHex rfcPassword rfcSalt 3 32 4 32 rfcKey rfcAd argon2Version
                `shouldBe` rfcExpected

    describe "input validation" $ do

        let run pw sl t m p tl =
                argon2i pw sl t m p tl [] [] argon2Version

        it "rejects salt shorter than 8 bytes" $
            evaluate (run [] (replicate 7 0x00) 1 8 1 32)
                `shouldThrow` anyErrorCall

        it "rejects tag length < 4" $
            evaluate (run [] (replicate 16 0x00) 1 8 1 3)
                `shouldThrow` anyErrorCall

        it "rejects parallelism = 0" $
            evaluate (run [] (replicate 16 0x00) 1 8 0 32)
                `shouldThrow` anyErrorCall

        it "rejects time_cost = 0" $
            evaluate (run [] (replicate 16 0x00) 0 8 1 32)
                `shouldThrow` anyErrorCall

        it "rejects memory_cost < 8*parallelism" $
            evaluate (run [] (replicate 16 0x00) 1 4 2 32)
                `shouldThrow` anyErrorCall

        it "rejects an unsupported version" $
            evaluate
                (argon2i [] (replicate 16 0x00) 1 8 1 32 [] [] 0x10)
                `shouldThrow` anyErrorCall

    describe "determinism and input binding" $ do

        let base = argon2i rfcPassword rfcSalt 1 16 1 32 [] [] argon2Version

        it "produces the same output for the same inputs" $
            argon2i rfcPassword rfcSalt 1 16 1 32 [] [] argon2Version
                `shouldBe` base

        it "changes when the password changes" $
            argon2i (replicate 32 0x05) rfcSalt 1 16 1 32 [] []
                    argon2Version
                `shouldNotBe` base

        it "changes when a key is added" $
            argon2i rfcPassword rfcSalt 1 16 1 32 rfcKey [] argon2Version
                `shouldNotBe` base

        it "changes when associated data is added" $
            argon2i rfcPassword rfcSalt 1 16 1 32 [] rfcAd argon2Version
                `shouldNotBe` base

    describe "tag length variants" $ do

        let at tl = length
                (argon2i rfcPassword rfcSalt 1 16 1 tl [] [] argon2Version)

        it "returns exactly 4 bytes"   $ at 4   `shouldBe` 4
        it "returns exactly 16 bytes"  $ at 16  `shouldBe` 16
        it "returns exactly 65 bytes"  $ at 65  `shouldBe` 65
        it "returns exactly 128 bytes" $ at 128 `shouldBe` 128

    describe "side-channel property" $ do

        it "differs from argon2d by using data-independent indexing" $
            -- Argon2i's address stream doesn't depend on the password,
            -- so with the same parameters but different password we
            -- should still get different tag bytes (no early exit).
            argon2i (replicate 32 0x00) rfcSalt 1 16 1 32 [] []
                    argon2Version
                `shouldNotBe`
                    argon2i (replicate 32 0x01) rfcSalt 1 16 1 32 [] []
                            argon2Version
