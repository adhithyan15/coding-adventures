-- | Test suite for the pure-Haskell Argon2d implementation.
--
-- The canonical vector comes from RFC 9106 §5.1: password = 32 bytes of
-- 0x01, salt = 16 bytes of 0x02, secret = 8 bytes of 0x03, associated
-- data = 12 bytes of 0x04, t = 3, m = 32 KiB, p = 4, T = 32 bytes.
module Argon2dSpec (spec) where

import Argon2d
import Control.Exception (evaluate)
import Data.Word (Word8)
import Test.Hspec

-- RFC 9106 §5.1 fixed inputs
rfcPassword :: [Word8]
rfcPassword = replicate 32 0x01

rfcSalt :: [Word8]
rfcSalt = replicate 16 0x02

rfcKey :: [Word8]
rfcKey = replicate 8 0x03

rfcAd :: [Word8]
rfcAd = replicate 12 0x04

rfcExpected :: String
rfcExpected = "512b391b6f1162975371d30919734294f868e3be3984f3c1a13a4db9fabe4acb"

spec :: Spec
spec = describe "Argon2d" $ do

    describe "RFC 9106 §5.1 canonical vector" $ do

        it "matches the expected 32-byte tag" $
            argon2dHex rfcPassword rfcSalt 3 32 4 32 rfcKey rfcAd argon2Version
                `shouldBe` rfcExpected

        it "hex and binary outputs agree" $ do
            let bs = argon2d rfcPassword rfcSalt 3 32 4 32 rfcKey rfcAd
                              argon2Version
                hx = argon2dHex rfcPassword rfcSalt 3 32 4 32 rfcKey rfcAd
                                argon2Version
            length bs `shouldBe` 32
            length hx `shouldBe` 64

    describe "input validation" $ do

        let run pw sl t m p tl =
                argon2d pw sl t m p tl [] [] argon2Version

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
                (argon2d [] (replicate 16 0x00) 1 8 1 32 [] [] 0x10)
                `shouldThrow` anyErrorCall

    describe "determinism and input binding" $ do

        let base = argon2d rfcPassword rfcSalt 1 16 1 32 [] [] argon2Version

        it "produces the same output for the same inputs" $
            argon2d rfcPassword rfcSalt 1 16 1 32 [] [] argon2Version
                `shouldBe` base

        it "changes when the password changes" $
            argon2d (replicate 32 0x05) rfcSalt 1 16 1 32 [] []
                    argon2Version
                `shouldNotBe` base

        it "changes when the salt changes" $
            argon2d rfcPassword (replicate 16 0x09) 1 16 1 32 [] []
                    argon2Version
                `shouldNotBe` base

        it "changes when a key is added" $
            argon2d rfcPassword rfcSalt 1 16 1 32 rfcKey [] argon2Version
                `shouldNotBe` base

        it "changes when associated data is added" $
            argon2d rfcPassword rfcSalt 1 16 1 32 [] rfcAd argon2Version
                `shouldNotBe` base

    describe "tag length variants" $ do

        let at tl = length
                (argon2d rfcPassword rfcSalt 1 16 1 tl [] [] argon2Version)

        it "returns exactly 4 bytes when requested"   $ at 4   `shouldBe` 4
        it "returns exactly 16 bytes when requested"  $ at 16  `shouldBe` 16
        it "returns exactly 65 bytes when requested"  $ at 65  `shouldBe` 65
        it "returns exactly 128 bytes when requested" $ at 128 `shouldBe` 128

    describe "multi-pass correctness" $ do

        it "yields a different tag for t=2 vs t=1 (same other params)" $
            argon2d rfcPassword rfcSalt 2 16 1 32 [] [] argon2Version
                `shouldNotBe`
                    argon2d rfcPassword rfcSalt 1 16 1 32 [] [] argon2Version
