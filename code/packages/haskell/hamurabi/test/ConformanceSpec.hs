module ConformanceSpec (spec) where

import Control.Exception (try)
import Control.Monad (forM_)
import Data.List (isPrefixOf)
import Data.Word (Word64)
import Hamurabi
import Test.Hspec

data TurnCheck = TurnCheck
  { turnNum        :: !Int
  , turnOrder      :: !TurnOrder
  , expectedState  :: !HamurabiState
  , expectedReport :: !String
  }

data RefusalCheck = RefusalCheck
  { refusalOrder :: !TurnOrder
  , expectedErr  :: !HamurabiError
  }

data TestCase = TestCase
  { caseId     :: !String
  , caseSeed   :: !(Maybe Word64)
  , initState  :: !HamurabiState
  , turns      :: ![TurnCheck]
  , refusal    :: !(Maybe RefusalCheck)
  }

unescapeReport :: String -> String
unescapeReport [] = []
unescapeReport ('\\':'n':xs) = '\n' : unescapeReport xs
unescapeReport (x:xs) = x : unescapeReport xs

parseStatus :: String -> GameStatus
parseStatus "Playing" = Playing
parseStatus "GameOverExtinction" = GameOverExtinction
parseStatus s
  | "Won(" `isPrefixOf` s =
      let inner = takeWhile (/= ')') (drop 4 s)
      in Won (read inner)
  | "Impeached(" `isPrefixOf` s =
      let part1 = dropWhile (/= '=') s
          num1Str = takeWhile (/= ',') (drop 1 part1)
          part2 = dropWhile (/= '=') (dropWhile (/= ',') s)
          num2Str = takeWhile (/= ')') (drop 1 part2)
      in Impeached (read num1Str) (read num2Str)
  | otherwise = error ("Unknown status string: " ++ s)

parseStateTokens :: [String] -> HamurabiState
parseStateTokens ws =
  -- ws: ["INIT"|"STATE", yr, pop, bus, acr, lp, hpa, re, sly, st, pss, ily, ps, ...status..., rngState, rngInc]
  let yr = read (ws !! 1)
      pop = read (ws !! 2)
      bus = read (ws !! 3)
      acr = read (ws !! 4)
      lp = read (ws !! 5)
      hpa = read (ws !! 6)
      re = read (ws !! 7)
      sly = read (ws !! 8)
      st = read (ws !! 9)
      pss = read (ws !! 10)
      ily = read (ws !! 11)
      ps = (read (ws !! 12) :: Int) /= 0
      len = length ws
      rngIncVal = read (ws !! (len - 1))
      rngStateVal = read (ws !! (len - 2))
      statusTokens = take (len - 2 - 13) (drop 13 ws)
      statusVal = parseStatus (unwords statusTokens)
  in HamurabiState
      { year = yr
      , population = pop
      , bushels = bus
      , acres = acr
      , landPrice = lp
      , harvestPerAcre = hpa
      , ratsEaten = re
      , starvedLastYear = sly
      , starvedTotal = st
      , percentStarvedSum = pss
      , immigrantsLastYear = ily
      , plagueStruck = ps
      , status = statusVal
      , rngState = rngStateVal
      , rngInc = rngIncVal
      }

parseError :: String -> HamurabiError
parseError "GameAlreadyEnded" = GameAlreadyEnded
parseError "InvalidOrderBounds" = InvalidOrderBounds
parseError "InsufficientBushelsForLand" = InsufficientBushelsForLand
parseError "InsufficientAcresToSell" = InsufficientAcresToSell
parseError "InsufficientBushelsToFeed" = InsufficientBushelsToFeed
parseError "InsufficientAcresToPlant" = InsufficientAcresToPlant
parseError "InsufficientPopulationToPlant" = InsufficientPopulationToPlant
parseError "InsufficientGrainForSeed" = InsufficientGrainForSeed
parseError other = error ("Unknown error name: " ++ other)

parseCorpus :: String -> [TestCase]
parseCorpus raw =
  let validLines = filter isMeaningful (lines raw)
  in parseCases validLines
  where
    isMeaningful l = case l of
      [] -> False
      ('#':_) -> False
      _ | "COUNT " `isPrefixOf` l -> False
      _ -> True

    parseCases [] = []
    parseCases (l:ls)
      | "CASE " `isPrefixOf` l =
          let ws = words l
              cId = ws !! 1
              seedStr = ws !! 2
              mbSeed = if seedStr == "null" then Nothing else Just (read seedStr)
              (caseBody, rest) = break ("CASE " `isPrefixOf`) ls
              tc = buildCase cId mbSeed caseBody
          in tc : parseCases rest
      | otherwise = parseCases ls

    buildCase cId mbSeed ls =
      go ls Nothing [] Nothing Nothing
      where
        go [] mbInit revTurns _ mbRef =
          case mbInit of
            Nothing -> error ("Case " ++ cId ++ " missing INIT line")
            Just ini -> TestCase cId mbSeed ini (reverse revTurns) mbRef
        go (line:rest) mbInit revTurns mbPendingTurn mbRef
          | "INIT " `isPrefixOf` line =
              let st = parseStateTokens (words line)
              in go rest (Just st) revTurns mbPendingTurn mbRef
          | "TURN " `isPrefixOf` line =
              let ws = words line
                  tNum = read (ws !! 1)
                  d = read (ws !! 2)
                  f = read (ws !! 3)
                  p = read (ws !! 4)
                  ord = TurnOrder { acresDelta = d, bushelsToFeed = f, acresToPlant = p }
              in go rest mbInit revTurns (Just (tNum, ord)) mbRef
          | "STATE " `isPrefixOf` line =
              let st = parseStateTokens (words line)
              in case mbPendingTurn of
                   Just (tNum, ord) ->
                     -- Look ahead to next line for REPORT
                     case rest of
                       (repLine:rest2) | "REPORT " `isPrefixOf` repLine ->
                         let rep = unescapeReport (drop 7 repLine)
                             tc = TurnCheck tNum ord st rep
                         in go rest2 mbInit (tc : revTurns) Nothing mbRef
                       _ -> error ("Expected REPORT line after STATE in case " ++ cId)
                   Nothing -> error ("STATE without TURN in case " ++ cId)
          | "REFUSAL " `isPrefixOf` line =
              let ws = words line
                  d = read (ws !! 1)
                  f = read (ws !! 2)
                  p = read (ws !! 3)
                  err = parseError (ws !! 4)
                  ord = TurnOrder { acresDelta = d, bushelsToFeed = f, acresToPlant = p }
                  ref = RefusalCheck ord err
              in go rest mbInit revTurns mbPendingTurn (Just ref)
          | otherwise = go rest mbInit revTurns mbPendingTurn mbRef

readCorpus :: IO String
readCorpus = do
  let candidates =
        [ "../../../specs/fixtures/hamurabi-v1/cases.txt"
        , "code/specs/fixtures/hamurabi-v1/cases.txt"
        ]
  go candidates
  where
    go [] = error "Could not find hamurabi-v1/cases.txt"
    go (p:ps) = do
      res <- try (readFile p) :: IO (Either IOError String)
      case res of
        Right content -> pure content
        Left _ -> go ps

spec :: Spec
spec = describe "hamurabi-v1 flat conformance corpus" $ do
  it "reproduces all 31 cases bit-for-bit" $ do
    corpusText <- readCorpus
    let casesList = parseCorpus corpusText
    length casesList `shouldBe` 31
    forM_ casesList $ \tc -> do
      case caseSeed tc of
        Just s | "full_" `isPrefixOf` caseId tc ->
          newGame s `shouldBe` initState tc
        _ -> pure ()

      let verifyTurns curState [] = pure curState
          verifyTurns curState (t:ts) = do
            case stepYear curState (turnOrder t) of
              Left err -> do
                expectationFailure (caseId tc ++ " turn " ++ show (turnNum t) ++ " failed: " ++ show err)
                pure curState
              Right nextSt -> do
                nextSt `shouldBe` expectedState t
                formatReport nextSt `shouldBe` expectedReport t
                verifyTurns nextSt ts

      _ <- verifyTurns (initState tc) (turns tc)

      case refusal tc of
        Nothing -> pure ()
        Just ref -> do
          validateOrder (initState tc) (refusalOrder ref) `shouldBe` Left (expectedErr ref)
          case stepYear (initState tc) (refusalOrder ref) of
            Left err -> err `shouldBe` expectedErr ref
            Right _ -> expectationFailure (caseId tc ++ " expected refusal but stepYear succeeded")
