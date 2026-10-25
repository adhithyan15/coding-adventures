module StarlarkCompilerSpec (spec) where

import qualified Data.Map.Strict as Map
import Data.Word (Word8)
import StarlarkCompiler
import Test.Hspec

spec :: Spec
spec = do
    describe "opcode table" $ do
        it "contains the complete ascending 61-opcode roster" $ do
            length allOps `shouldBe` 61
            map opByte allOps `shouldBe` expectedBytes
            map opName (take 6 allOps) `shouldBe`
                ["LoadConst", "Pop", "Dup", "LoadNone", "LoadTrue", "LoadFalse"]

        it "round-trips every assigned byte and rejects every hole" $ do
            [opFromByte byte | byte <- [0 .. 255]]
                `shouldBe` [lookup byte assigned | byte <- [0 .. 255]]
            map (opFromByte . opByte) allOps `shouldBe` map Just allOps

        it "reports exact names and categories" $ do
            map (\op -> (opName op, opCategory op)) representatives `shouldBe`
                [ ("LoadConst", Stack)
                , ("LoadName", Variable)
                , ("RShift", Arithmetic)
                , ("Not", Comparison)
                , ("JumpIfTrue", ControlFlow)
                , ("Return", Function)
                , ("DictSet", Collection)
                , ("LoadSlice", SubscriptAttribute)
                , ("UnpackSequence", Iteration)
                , ("ImportFrom", Module)
                , ("Print", InputOutput)
                , ("Halt", VMControl)
                ]

    describe "operator maps" $ do
        it "exposes all four exact domains" $ do
            Map.toList binaryOpMap `shouldMatchList`
                [("+", Add), ("-", Sub), ("*", Mul), ("/", Div), ("//", FloorDiv), ("%", Mod), ("**", Power), ("&", BitAnd), ("|", BitOr), ("^", BitXor), ("<<", LShift), (">>", RShift)]
            Map.toList compareOpMap `shouldMatchList`
                [("==", CmpEq), ("!=", CmpNe), ("<", CmpLt), (">", CmpGt), ("<=", CmpLe), (">=", CmpGe), ("in", CmpIn), ("not in", CmpNotIn)]
            Map.toList augmentedAssignMap `shouldMatchList`
                [("+=", Add), ("-=", Sub), ("*=", Mul), ("/=", Div), ("//=", FloorDiv), ("%=", Mod), ("**=", Power), ("&=", BitAnd), ("|=", BitOr), ("^=", BitXor), ("<<=", LShift), (">>=", RShift)]
            Map.toList unaryOpMap `shouldMatchList` [("-", Negate), ("~", BitNot)]

        it "looks up exact spellings without normalization" $ do
            binaryOpcode "+" `shouldBe` Just Add
            compareOpcode "not in" `shouldBe` Just CmpNotIn
            augmentedAssignOpcode "**=" `shouldBe` Just Power
            unaryOpcode "~" `shouldBe` Just BitNot
            map binaryOpcode ["@", "++", "＋"] `shouldBe` replicate 3 Nothing
            map compareOpcode ["IN", "not  in"] `shouldBe` replicate 2 Nothing
            augmentedAssignOpcode ":=" `shouldBe` Nothing
            map unaryOpcode ["+", "not"] `shouldBe` replicate 2 Nothing

        it "keeps persistent maps independent after edits" $ do
            let exercise original = do
                    let changed = Map.insert "new" Halt (Map.deleteMin original)
                    Map.member "new" original `shouldBe` False
                    Map.member "new" changed `shouldBe` True
                    Map.size original `shouldBe` Map.size changed
            mapM_ exercise [binaryOpMap, compareOpMap, augmentedAssignMap, unaryOpMap]

assigned :: [(Word8, Op)]
assigned = [(opByte op, op) | op <- allOps]

expectedBytes :: [Word8]
expectedBytes =
    [ 0x01, 0x02, 0x03, 0x04, 0x05, 0x06
    , 0x10, 0x11, 0x12, 0x13, 0x14, 0x15
    , 0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2A, 0x2B, 0x2C, 0x2D
    , 0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38
    , 0x40, 0x41, 0x42, 0x43, 0x44
    , 0x50, 0x51, 0x52, 0x53
    , 0x60, 0x61, 0x62, 0x63, 0x64
    , 0x70, 0x71, 0x72, 0x73, 0x74
    , 0x80, 0x81, 0x82
    , 0x90, 0x91, 0xA0, 0xFF
    ]

representatives :: [Op]
representatives =
    [ LoadConst, LoadName, RShift, Not, JumpIfTrue, Return
    , DictSet, LoadSlice, UnpackSequence, ImportFrom, Print, Halt
    ]
