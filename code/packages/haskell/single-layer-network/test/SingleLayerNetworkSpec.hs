module SingleLayerNetworkSpec (spec) where

import Test.Hspec hiding (fit)
import SingleLayerNetwork

near :: Double -> Double -> Bool
near actual expected = abs (actual - expected) <= 1e-6

spec :: Spec
spec = do
    it "exposes one-epoch matrix gradients" $ do
        let result = trainOneEpochWithMatrices [[1.0, 2.0]] [[3.0, 5.0]] [[0.0, 0.0], [0.0, 0.0]] [0.0, 0.0] 0.1 Linear
        fmap predictions result `shouldBe` Right [[0.0, 0.0]]
        fmap errors result `shouldBe` Right [[-3.0, -5.0]]
        fmap weightGradients result `shouldBe` Right [[-3.0, -5.0], [-6.0, -10.0]]
        fmap ((`near` 0.3) . (!! 0) . (!! 0) . nextWeights) result `shouldBe` Right True
        fmap ((`near` 1.0) . (!! 1) . (!! 1) . nextWeights) result `shouldBe` Right True

    it "fits m inputs to n outputs" $ do
        let result = fit
                (newModel 3 2 Linear)
                [[0.0, 0.0, 1.0], [1.0, 2.0, 1.0], [2.0, 1.0, 1.0]]
                [[1.0, -1.0], [3.0, 2.0], [4.0, 1.0]]
                0.05
                500
        fmap historyImproved result `shouldBe` Right True
        fmap predictedOutputCount result `shouldBe` Right (Right 2)

historyImproved :: (Model, [TrainingStep]) -> Bool
historyImproved (_, history) =
    case history of
        firstStep:_ ->
            case reverse history of
                lastStep:_ -> loss lastStep < loss firstStep
                [] -> False
        [] -> False

predictedOutputCount :: (Model, [TrainingStep]) -> Either String Int
predictedOutputCount (model, _) =
    case predict model [[1.0, 1.0, 1.0]] of
        Right (row:_) -> Right (length row)
        Right [] -> Left "prediction returned no rows"
        Left err -> Left err
