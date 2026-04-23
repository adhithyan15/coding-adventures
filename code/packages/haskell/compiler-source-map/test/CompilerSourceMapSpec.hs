module CompilerSourceMapSpec (spec) where

import Test.Hspec

import CompilerSourceMap

spec :: Spec
spec = do
    describe "formatSourcePosition" $
        it "renders file, line, column, and length" $
            formatSourcePosition samplePosition `shouldBe` "hello.bf:1:3 (len=1)"

    describe "SourceToAst" $ do
        it "looks up positions by AST node id" $ do
            let mapping =
                    addSourceToAst samplePosition 42 $
                        addSourceToAst (SourcePosition "hello.bf" 1 4 1) 43 emptySourceToAst
            lookupSourceByAstNodeId 42 mapping `shouldBe` Just samplePosition

        it "matches positions by file, line, and column" $ do
            let mapping = addSourceToAst samplePosition 42 emptySourceToAst
            lookupAstNodeIdBySourcePosition (samplePosition{sourcePositionLength = 4}) mapping
                `shouldBe` Just 42

    describe "AstToIr" $
        it "finds the originating AST node from an IR id" $ do
            let mapping =
                    addAstToIr 42 [7, 8, 9, 10] $
                        addAstToIr 43 [11] emptyAstToIr
            lookupAstNodeIdByIrId 9 mapping `shouldBe` Just 42

    describe "IrToIr" $ do
        it "tracks replacement mappings" $ do
            let segment = addIrMapping 7 [100] $ addIrMapping 8 [101] (emptyIrToIr "contraction")
            lookupNewIrIdsByOriginalId 8 segment `shouldBe` Just [101]

        it "treats deleted instructions as absent" $ do
            let segment = addIrDeletion 9 (emptyIrToIr "dead-store")
            lookupNewIrIdsByOriginalId 9 segment `shouldBe` Nothing

    describe "IrToMachineCode" $
        it "maps byte offsets back to IR ids" $ do
            let segment =
                    addIrToMachineCode 7 0 4 $
                        addIrToMachineCode 8 4 4 emptyIrToMachineCode
            lookupIrIdByMachineCodeOffset 6 segment `shouldBe` Just 8

    describe "SourceMapChain" $ do
        it "walks source positions forward to machine code" $ do
            let expected =
                    [ IrToMachineCodeEntry 100 0 4
                    , IrToMachineCodeEntry 101 4 4
                    ]
            sourceToMachineCode completeChain samplePosition `shouldBe` Just expected

        it "walks machine code offsets back to source positions" $
            machineCodeToSource completeChain 5 `shouldBe` Just samplePosition

        it "returns Nothing when all IR instructions are deleted" $ do
            let deletedPass = addIrDeletion 7 $ addIrDeletion 8 (emptyIrToIr "erase")
                chain = addOptimizerPass deletedPass baseChain
            sourceToMachineCode chain samplePosition `shouldBe` Nothing

samplePosition :: SourcePosition
samplePosition = SourcePosition "hello.bf" 1 3 1

baseChain :: SourceMapChain
baseChain =
    emptySourceMapChain
        { sourceMapChainSourceToAst = addSourceToAst samplePosition 42 emptySourceToAst
        , sourceMapChainAstToIr = addAstToIr 42 [7, 8] emptyAstToIr
        }

completeChain :: SourceMapChain
completeChain =
    setIrToMachineCode machineCodeSegment $
        addOptimizerPass optimizerPass baseChain

optimizerPass :: IrToIr
optimizerPass =
    addIrMapping 7 [100] $
        addIrMapping 8 [101] (emptyIrToIr "contraction")

machineCodeSegment :: IrToMachineCode
machineCodeSegment =
    addIrToMachineCode 100 0 4 $
        addIrToMachineCode 101 4 4 emptyIrToMachineCode
