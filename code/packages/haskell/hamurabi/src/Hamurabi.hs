{-# LANGUAGE BangPatterns #-}

-- | Pure, deterministic simulation core for the seminal 1968 mainframe resource
-- allocation game (GAME01).
module Hamurabi
  ( Rating(..)
  , GameStatus(..)
  , HamurabiError(..)
  , HamurabiState(..)
  , TurnOrder(..)
  , newGame
  , validateOrder
  , stepYear
  , formatReport
  ) where

import Control.Monad (when)
import Data.Bits (rotateR, shiftL, shiftR, xor, (.&.), (.|.))
import Data.Int (Int32)
import Data.Word (Word8, Word32, Word64)

-- | Rating classification awarded to ruler upon completing a 10-year term.
data Rating
  = Charlemagne
  | Acre
  | Nero
  | Disaster
  deriving (Eq, Show, Read, Enum, Bounded)

-- | Simulation lifecycle status.
data GameStatus
  = Playing
  | Won !Rating
  | Impeached !Word32 !Word32 -- ^ starved_this_turn, percent
  | GameOverExtinction
  deriving (Eq, Show, Read)

-- | Closed enumeration of order rejection errors.
data HamurabiError
  = GameAlreadyEnded
  | InvalidOrderBounds
  | InsufficientBushelsForLand
  | InsufficientAcresToSell
  | InsufficientBushelsToFeed
  | InsufficientAcresToPlant
  | InsufficientPopulationToPlant
  | InsufficientGrainForSeed
  deriving (Eq, Show, Read, Enum, Bounded)

-- | Immutable value record capturing the complete simulation state.
data HamurabiState = HamurabiState
  { year               :: !Word8
  , population         :: !Word32
  , bushels            :: !Word32
  , acres              :: !Word32
  , landPrice          :: !Word32
  , harvestPerAcre     :: !Word32
  , ratsEaten          :: !Word32
  , starvedLastYear    :: !Word32
  , starvedTotal       :: !Word32
  , percentStarvedSum  :: !Word32
  , immigrantsLastYear :: !Word32
  , plagueStruck       :: !Bool
  , status             :: !GameStatus
  , rngState           :: !Word64
  , rngInc             :: !Word64
  } deriving (Eq, Show)

-- | Player orders for a single year.
data TurnOrder = TurnOrder
  { acresDelta    :: !Int32
  , bushelsToFeed :: !Word32
  , acresToPlant  :: !Word32
  } deriving (Eq, Show)

-- ── PRNG Internal Primitives (PCG32 XSH RR 64/32) ──────────────────────────

pcgMultiplier :: Word64
pcgMultiplier = 6364136223846793005

hamurabiStreamId :: Word64
hamurabiStreamId = 54

pcg32Step :: Word64 -> Word64 -> (Word32, Word64)
pcg32Step !s !inc =
  let !oldState = s
      !newState = oldState * pcgMultiplier + inc
      !xorshifted = fromIntegral (((oldState `shiftR` 18) `xor` oldState) `shiftR` 27) :: Word32
      !rot = fromIntegral (oldState `shiftR` 59) :: Int
      !out = rotateR xorshifted rot
  in (out, newState)

pcg32NextIntInRange :: Word32 -> Word32 -> Word64 -> Word64 -> (Word32, Word64)
pcg32NextIntInRange !minVal !maxVal !s !inc =
  let !spanVal = fromIntegral (maxVal - minVal + 1) :: Word64
      !threshold = ((1 `shiftL` 32) - spanVal) `mod` spanVal
      go !st =
        let (!r32, !st') = pcg32Step st inc
            !r = fromIntegral r32 :: Word64
        in if r >= threshold
           then (minVal + fromIntegral (r `mod` spanVal), st')
           else go st'
  in go s

-- ── Simulation Engine API ───────────────────────────────────────────────────

-- | Initializes a new, deterministic 10-year reign for ruler Hamurabi.
newGame :: Word64 -> HamurabiState
newGame !seed =
  let !inc = (hamurabiStreamId `shiftL` 1) .|. 1 -- 109
      !s0 = 0 :: Word64
      !s1 = s0 * pcgMultiplier + inc
      !s2 = s1 + seed
      !s3 = s2 * pcgMultiplier + inc
      (!price, !s4) = pcg32NextIntInRange 17 26 s3 inc
  in HamurabiState
      { year = 1
      , population = 100
      , bushels = 2800
      , acres = 1000
      , landPrice = price
      , harvestPerAcre = 3
      , ratsEaten = 200
      , starvedLastYear = 0
      , starvedTotal = 0
      , percentStarvedSum = 0
      , immigrantsLastYear = 5
      , plagueStruck = False
      , status = Playing
      , rngState = s4
      , rngInc = inc
      }

-- | Validates player orders against current state constraints without advancing state.
validateOrder :: HamurabiState -> TurnOrder -> Either HamurabiError ()
validateOrder !st !ord
  -- 1. Status Check
  | status st /= Playing = Left GameAlreadyEnded
  -- State invariant checks (VER04m state_inv & CWE-400 / CWE-835)
  | (rngInc st .&. 1) /= 1 = Left InvalidOrderBounds
  | year st < 1 || year st > 10 = Left InvalidOrderBounds
  | landPrice st < 17 || landPrice st > 26 = Left InvalidOrderBounds
  | harvestPerAcre st < 1 || harvestPerAcre st > 5 = Left InvalidOrderBounds
  | otherwise = do
      -- 2. Bounds & Overflow Guard on Land Trade
      let delta = toInteger (acresDelta ord)
          price = toInteger (landPrice st)
          curAcres = toInteger (acres st)
          curBushels = toInteger (bushels st)
          maxU32 = toInteger (maxBound :: Word32)

      when (delta > 0) $ do
        let cost = delta * price
        when (cost > curBushels) $ Left InsufficientBushelsForLand

      when (delta < 0) $ do
        let sellCount = negate delta
        when (sellCount > curAcres) $ Left InsufficientAcresToSell

      -- 3. Land Trade Balances
      let acresAfterTrade =
            if delta >= 0
            then curAcres + delta
            else curAcres - negate delta

      when (acresAfterTrade < 0 || acresAfterTrade > maxU32) $
        Left InvalidOrderBounds

      bushelsAfterTrade <-
        if delta > 0
        then pure (curBushels - (delta * price))
        else if delta < 0
        then do
          let sellCount = negate delta
              revenue = sellCount * price
              total = curBushels + revenue
          if total > maxU32
            then Left InvalidOrderBounds
            else pure total
        else pure curBushels

      -- 4. Food Allocation Check
      let feed = toInteger (bushelsToFeed ord)
      when (feed > bushelsAfterTrade) $
        Left InsufficientBushelsToFeed
      let bushelsAfterFeed = bushelsAfterTrade - feed

      -- 5. Planting Land & Labor Checks
      let plant = toInteger (acresToPlant ord)
      when (plant > acresAfterTrade) $
        Left InsufficientAcresToPlant
      let maxLabor = toInteger (population st) * 10
      when (plant > maxLabor) $
        Left InsufficientPopulationToPlant

      -- 6. Planting Seed Check: ceil(acres / 2) = (acres + 1) / 2
      let seedNeeded = (plant + 1) `div` 2
      when (seedNeeded > bushelsAfterFeed) $
        Left InsufficientGrainForSeed
      let bushelsAfterSeed = bushelsAfterFeed - seedNeeded

      -- 7. Maximum Harvest Overflow Guard (CWE-190)
      let maxHarvest = plant * 5
          maxStored = bushelsAfterSeed + maxHarvest
      when (maxStored > maxU32) $
        Left InvalidOrderBounds

      pure ()

-- | Advances the simulation by exactly one year according to player orders.
stepYear :: HamurabiState -> TurnOrder -> Either HamurabiError HamurabiState
stepYear !st !ord = do
  validateOrder st ord
  let !delta = acresDelta ord
      !price = landPrice st
      !curAcres = acres st
      !curBushels = bushels st

      !acresAfterTrade =
        if delta >= 0
        then curAcres + fromIntegral delta
        else curAcres - fromIntegral (negate delta)

      !bushelsAfterTrade =
        if delta > 0
        then curBushels - (fromIntegral delta * price)
        else if delta < 0
        then curBushels + (fromIntegral (negate delta) * price)
        else curBushels

      !bushelsAfterFeed = bushelsAfterTrade - bushelsToFeed ord
      !seedNeeded = (acresToPlant ord + 1) `div` 2
      !bushelsAfterSeed = bushelsAfterFeed - seedNeeded

      !inc = rngInc st
      -- Step 1: Harvest Yield Draw [1, 5]
      (!harvestYield, !s1) = pcg32NextIntInRange 1 5 (rngState st) inc
      !totalHarvest = toInteger (acresToPlant ord) * toInteger harvestYield
      !bushelsStored = toInteger bushelsAfterSeed + totalHarvest

      -- Step 2: Rat Infestation Draws [1, 100]
      (!ratRoll, !s2) = pcg32NextIntInRange 1 100 s1 inc
      (!ratsDestroyed, !s3) =
        if ratRoll <= 40
        then
          let (!ratFactor, !sRat) = pcg32NextIntInRange 1 5 s2 inc
              !eaten = bushelsStored `div` toInteger ratFactor
              !clamped = fromIntegral (min (toInteger (maxBound :: Word32)) eaten) :: Word32
          in (clamped, sRat)
        else (0, s2)

      !bushelsRemaining = max 0 (bushelsStored - toInteger ratsDestroyed)
      !bushelsFinal = fromIntegral (min (toInteger (maxBound :: Word32)) bushelsRemaining) :: Word32

      -- Step 3: Plague Draw [1, 100]
      (!plagueRoll, !s4) = pcg32NextIntInRange 1 100 s3 inc
      !plague = plagueRoll <= 15

      -- Step 4: Starvation & Impeachment Evaluation
      !peopleFed = bushelsToFeed ord `div` 20
      (!starved, !survivors0) =
        if peopleFed >= population st
        then (0, population st)
        else (population st - peopleFed, peopleFed)

      !percentStarved =
        if population st > 0
        then fromIntegral (min 100 ((toInteger starved * 100) `div` toInteger (population st))) :: Word32
        else 0

      !newStarvedTotal =
        fromIntegral (min (toInteger (maxBound :: Word32)) (toInteger (starvedTotal st) + toInteger starved)) :: Word32
      !newPercentSum =
        fromIntegral (min (toInteger (maxBound :: Word32)) (toInteger (percentStarvedSum st) + toInteger percentStarved)) :: Word32

      !status0
        | toInteger starved * 100 > toInteger (population st) * 45 =
            Impeached starved percentStarved
        | survivors0 == 0 =
            GameOverExtinction
        | otherwise =
            Playing

      -- Step 5: Plague Resolution
      !survivors1 =
        if plague && status0 == Playing
        then survivors0 `div` 2
        else survivors0

      -- Step 6: Immigration Draw [1, 5]
      (!immFactor, !s5) = pcg32NextIntInRange 1 5 s4 inc
      !immigrants =
        if starved == 0 && status0 == Playing
        then
          let !acresTerm = toInteger acresAfterTrade * 20
              !inner = acresTerm + toInteger bushelsFinal
              !numerator = toInteger immFactor * inner
              !denominator = 100 * toInteger (population st)
          in if denominator == 0
             then 0
             else fromIntegral (min (toInteger (maxBound :: Word32)) ((numerator `div` denominator) + 1)) :: Word32
        else 0

      !finalPopulation =
        fromIntegral (min (toInteger (maxBound :: Word32)) (toInteger survivors1 + toInteger immigrants)) :: Word32

      -- Step 7: Next Year Land Price Draw [17, 26]
      (!nextLandPrice, !s6) = pcg32NextIntInRange 17 26 s5 inc

      -- Step 8: Turn Advance & Rating
      !nextYear = fromIntegral (min 255 (toInteger (year st) + 1)) :: Word8
      !finalStatus =
        if nextYear == 11 && status0 == Playing
        then
          let !avgStarved = newPercentSum `div` 10
              !acresPerPerson = if finalPopulation > 0 then acresAfterTrade `div` finalPopulation else 0
              !rating
                | avgStarved <= 3 && acresPerPerson >= 10 = Charlemagne
                | avgStarved <= 10 && acresPerPerson >= 9 = Acre
                | avgStarved <= 33 && acresPerPerson >= 7 = Nero
                | otherwise                               = Disaster
          in Won rating
        else status0

  pure HamurabiState
    { year = nextYear
    , population = finalPopulation
    , bushels = bushelsFinal
    , acres = acresAfterTrade
    , landPrice = nextLandPrice
    , harvestPerAcre = harvestYield
    , ratsEaten = ratsDestroyed
    , starvedLastYear = starved
    , starvedTotal = newStarvedTotal
    , percentStarvedSum = newPercentSum
    , immigrantsLastYear = immigrants
    , plagueStruck = plague
    , status = finalStatus
    , rngState = s6
    , rngInc = inc
    }

-- | Formats the annual teletype report or final evaluation in authentic 1970s teletype styling.
formatReport :: HamurabiState -> String
formatReport !s = case status s of
  Impeached starved _ ->
    "YOU STARVED " ++ show starved ++ " PEOPLE IN ONE YEAR!!!\n\
    \DUE TO THIS EXTREME MISMANAGEMENT YOU HAVE BEEN IMPEACHED AND THROWN OUT OF OFFICE!\n"
  GameOverExtinction ->
    "EVERYONE HAS DIED. THE CITY OF LAGASH IS EXTINCT!\n"
  Won rating ->
    let avgStarved = percentStarvedSum s `div` 10
        acresPerPerson = if population s > 0 then acres s `div` population s else 0
        ratingMsg = case rating of
          Charlemagne ->
            "A FANTASTIC PERFORMANCE! CHARLEMAGNE, DISRAELI, AND JEFFERSON COMBINED COULD NOT HAVE DONE BETTER!"
          Acre ->
            "YOUR PERFORMANCE COULD HAVE BEEN WORSE, BUT YOU LEFT THEM REASONABLE ACREAGE."
          Nero ->
            "YOUR PERFORMANCE LEAVES MUCH TO BE DESIRED. NERO WOULD HAVE BEEN PROUD OF YOU."
          Disaster ->
            "YOU DID SO POORLY YOU HAVE BEEN DECLARED AN OFFICIAL NATIONAL DISASTER!"
    in "IN YOUR 10-YEAR TERM OF OFFICE, " ++ show avgStarved ++ " PERCENT OF THE\n\
       \POPULATION STARVED PER YEAR ON AVERAGE.\n\
       \A TOTAL OF " ++ show (starvedTotal s) ++ " PEOPLE DIED!!\n\
       \YOU STARTED WITH 10 ACRES PER PERSON AND ENDED WITH\n\
       \" ++ show acresPerPerson ++ " ACRES PER PERSON.\n\n\
       \" ++ ratingMsg ++ "\n"
  Playing ->
    let base = "HAMURABI: I BEG TO REPORT TO YOU,\n\
               \IN YEAR " ++ show (year s) ++ ", " ++ show (starvedLastYear s) ++ " PEOPLE STARVED, " ++ show (immigrantsLastYear s) ++ " CAME TO THE CITY.\n"
        plagueMsg = if plagueStruck s then "A HORRIBLE PLAGUE STRUCK! HALF THE PEOPLE DIED.\n" else ""
        rest = "POPULATION IS NOW " ++ show (population s) ++ ".\n\
               \THE CITY NOW OWNS " ++ show (acres s) ++ " ACRES.\n\
               \YOU HARVESTED " ++ show (harvestPerAcre s) ++ " BUSHELS PER ACRE.\n\
               \RATS ATE " ++ show (ratsEaten s) ++ " BUSHELS.\n\
               \YOU NOW HAVE " ++ show (bushels s) ++ " BUSHELS IN STORE.\n\
               \LAND IS TRADING AT " ++ show (landPrice s) ++ " BUSHELS PER ACRE.\n"
    in base ++ plagueMsg ++ rest
