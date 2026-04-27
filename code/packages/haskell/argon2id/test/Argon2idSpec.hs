-- | Test suite for the pure-Haskell Argon2id implementation.
--
-- Canonical vector comes from RFC 9106 §5.3.
module Argon2idSpec (spec) where

import Argon2id
import Control.Exception (evaluate)
import Data.Word (Word8)
import Test.Hspec

rfcPassword, rfcSalt, rfcKey, rfcAd :: [Word8]
rfcPassword = replicate 32 0x01
rfcSalt     = replicate 16 0x02
rfcKey      = replicate 8  0x03
rfcAd       = replicate 12 0x04

rfcExpected :: String
rfcExpected = "0d640df58d78766c08c037a34a8b53c9d01ef0452d75b65eb52520e96b01e659"

spec :: Spec
spec = describe "Argon2id" $ do

    describe "RFC 9106 §5.3 canonical vector" $ do

        it "matches the expected 32-byte tag" $
            argon2idHex rfcPassword rfcSalt 3 32 4 32 rfcKey rfcAd argon2Version
                `shouldBe` rfcExpected

    describe "input validation" $ do

        let run pw sl t m p tl =
                argon2id pw sl t m p tl [] [] argon2Version

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
                (argon2id [] (replicate 16 0x00) 1 8 1 32 [] [] 0x10)
                `shouldThrow` anyErrorCall

    describe "determinism and input binding" $ do

        let base = argon2id rfcPassword rfcSalt 1 16 1 32 [] [] argon2Version

        it "produces the same output for the same inputs" $
            argon2id rfcPassword rfcSalt 1 16 1 32 [] [] argon2Version
                `shouldBe` base

        it "changes when the password changes" $
            argon2id (replicate 32 0x05) rfcSalt 1 16 1 32 [] []
                     argon2Version
                `shouldNotBe` base

        it "changes when the salt changes" $
            argon2id rfcPassword (replicate 16 0x09) 1 16 1 32 [] []
                     argon2Version
                `shouldNotBe` base

        it "changes when a key is added" $
            argon2id rfcPassword rfcSalt 1 16 1 32 rfcKey [] argon2Version
                `shouldNotBe` base

        it "changes when associated data is added" $
            argon2id rfcPassword rfcSalt 1 16 1 32 [] rfcAd argon2Version
                `shouldNotBe` base

    describe "tag length variants" $ do

        let at tl = length
                (argon2id rfcPassword rfcSalt 1 16 1 tl [] [] argon2Version)

        it "returns exactly 4 bytes"   $ at 4   `shouldBe` 4
        it "returns exactly 16 bytes"  $ at 16  `shouldBe` 16
        it "returns exactly 65 bytes"  $ at 65  `shouldBe` 65
        it "returns exactly 128 bytes" $ at 128 `shouldBe` 128

    describe "hybrid distinctness" $ do

        it "produces a different tag than argon2d/i for the RFC inputs" $
            -- If our hybrid rule were off we'd trivially collide with
            -- one of the siblings; the §5.3 vector pins argon2id down.
            argon2idHex rfcPassword rfcSalt 3 32 4 32 rfcKey rfcAd
                        argon2Version
                `shouldNotBe`
                    "512b391b6f1162975371d30919734294f868e3be3984f3c1a13a4db9fabe4acb"
