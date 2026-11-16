module HamurabiSpec (spec) where

import Hamurabi
import Test.Hspec

spec :: Spec
spec = do
  describe "Hamurabi unit tests" $ do
    it "newGame initializes canonical 1968 baseline parameters" $ do
      let s = newGame 42
      year s `shouldBe` 1
      population s `shouldBe` 100
      bushels s `shouldBe` 2800
      acres s `shouldBe` 1000
      harvestPerAcre s `shouldBe` 3
      ratsEaten s `shouldBe` 200
      starvedLastYear s `shouldBe` 0
      starvedTotal s `shouldBe` 0
      percentStarvedSum s `shouldBe` 0
      immigrantsLastYear s `shouldBe` 5
      plagueStruck s `shouldBe` False
      status s `shouldBe` Playing
      landPrice s `shouldSatisfy` (\p -> p >= 17 && p <= 26)
      (rngInc s `rem` 2) `shouldBe` 1

    it "validateOrder accepts valid turn orders" $ do
      let s = newGame 42
          validOrder = TurnOrder { acresDelta = 0, bushelsToFeed = 2000, acresToPlant = 500 }
      validateOrder s validOrder `shouldBe` Right ()

    it "validateOrder rejects orders when game has ended" $ do
      let sWon = (newGame 42) { status = Won Charlemagne }
          sImpeached = (newGame 42) { status = Impeached 50 50 }
          sExtinct = (newGame 42) { status = GameOverExtinction }
          ord = TurnOrder { acresDelta = 0, bushelsToFeed = 2000, acresToPlant = 500 }
      validateOrder sWon ord `shouldBe` Left GameAlreadyEnded
      validateOrder sImpeached ord `shouldBe` Left GameAlreadyEnded
      validateOrder sExtinct ord `shouldBe` Left GameAlreadyEnded

    it "validateOrder enforces state invariants (VER04m / CWE-400)" $ do
      let s0 = newGame 42
          ord = TurnOrder { acresDelta = 0, bushelsToFeed = 1000, acresToPlant = 500 }
      validateOrder (s0 { rngInc = 108 }) ord `shouldBe` Left InvalidOrderBounds
      validateOrder (s0 { year = 0 }) ord `shouldBe` Left InvalidOrderBounds
      validateOrder (s0 { year = 11 }) ord `shouldBe` Left InvalidOrderBounds
      validateOrder (s0 { landPrice = 16 }) ord `shouldBe` Left InvalidOrderBounds
      validateOrder (s0 { landPrice = 27 }) ord `shouldBe` Left InvalidOrderBounds
      validateOrder (s0 { harvestPerAcre = 0 }) ord `shouldBe` Left InvalidOrderBounds
      validateOrder (s0 { harvestPerAcre = 6 }) ord `shouldBe` Left InvalidOrderBounds

    it "validateOrder rejects insufficient resources in canonical order" $ do
      let s = newGame 42 -- bushels=2800, acres=1000, pop=100
      -- 1. Insufficient bushels to buy land
      let ordBuyTooMuch = TurnOrder { acresDelta = 200, bushelsToFeed = 0, acresToPlant = 0 }
      validateOrder s ordBuyTooMuch `shouldBe` Left InsufficientBushelsForLand

      -- 2. Insufficient acres to sell
      let ordSellTooMuch = TurnOrder { acresDelta = -1001, bushelsToFeed = 0, acresToPlant = 0 }
      validateOrder s ordSellTooMuch `shouldBe` Left InsufficientAcresToSell

      -- 3. Insufficient bushels to feed
      let ordFeedTooMuch = TurnOrder { acresDelta = 0, bushelsToFeed = 2801, acresToPlant = 0 }
      validateOrder s ordFeedTooMuch `shouldBe` Left InsufficientBushelsToFeed

      -- 4. Insufficient acres to plant
      let ordPlantTooMuchAcres = TurnOrder { acresDelta = 0, bushelsToFeed = 0, acresToPlant = 1001 }
      validateOrder s ordPlantTooMuchAcres `shouldBe` Left InsufficientAcresToPlant

      -- 5. Insufficient population to plant
      let sFewPeople = s { population = 10, acres = 1000, bushels = 2800 }
          ordPlantTooMuchPop = TurnOrder { acresDelta = 0, bushelsToFeed = 0, acresToPlant = 200 }
      validateOrder sFewPeople ordPlantTooMuchPop `shouldBe` Left InsufficientPopulationToPlant

      -- 6. Insufficient grain for seed
      let ordPlantNoGrain = TurnOrder { acresDelta = 0, bushelsToFeed = 2800, acresToPlant = 100 }
      validateOrder s ordPlantNoGrain `shouldBe` Left InsufficientGrainForSeed

    it "stepYear advances simulation state and handles ratings" $ do
      let s = newGame 2
          ord = TurnOrder { acresDelta = 0, bushelsToFeed = 2000, acresToPlant = 1000 }
      case stepYear s ord of
        Left err -> expectationFailure ("stepYear failed: " ++ show err)
        Right s' -> do
          year s' `shouldBe` 2
          population s' `shouldBe` 103
          bushels s' `shouldBe` 4300
          status s' `shouldBe` Playing

    it "formatReport produces expected authentic teletype text" $ do
      let s = (newGame 42) { status = GameOverExtinction }
      formatReport s `shouldBe` "EVERYONE HAS DIED. THE CITY OF LAGASH IS EXTINCT!\n"
