-- | The honest runner: verdict honesty, re-verification, CI redaction.
module RunnerSpec (tests) where

import Control.DeepSeq (NFData (..))
import Control.Exception
import Data.IORef
import Data.Maybe (isJust, isNothing)
import Data.Text (Text)
import qualified Data.Text as T

import CodingAdventures.TestBootstrap
import Harness

off, on, unset :: Options
off = defaultOptions {optCi = CiOff}
on = defaultOptions {optCi = CiOn}
unset = defaultOptions

isArg :: PbtError -> Bool
isArg (ArgumentError m) = length m > 0 && length (displayException (ArgumentError m)) > 0
isArg _ = False

le100 :: Integer -> PropM Bool
le100 = pureProp (<= 100)

-- | A witness whose rendering explodes.
newtype Hostile = Hostile Int

instance NFData Hostile where rnf (Hostile n) = rnf n

instance Canonical Hostile where toVal (Hostile _) = error "toVal boom"

data Boom = Boom deriving (Show)

instance Exception Boom

tests :: [Test]
tests =
  [ test "a holding property passes" $ do
      v <- checkPure 1 (int 0 100) off (<= 100)
      assertTrue "passed" (vPassed v)
      assertEq "cases" 100 (vCasesRun v)
      assertEq "no failure id" Nothing (vFailureId v)
      assertEq "no message" Nothing (vMessage v)
  , test "a falsified property shrinks to the minimal witness" $ do
      v <- check 42 (int 0 100000) off le100
      assertFalse "passed" (vPassed v)
      assertEq "id" (Just failureCounterexample) (vFailureId v)
      assertEq "minimal" (Just "101") (vMinimal v)
      assertTrue "initial rendered" (isJust (vCounterexample v))
      assertEq "seed visible locally" (Just 42) (vSeed v)
      assertTrue "shrank" (vShrinkSteps v > 0)
      assertTrue "message names the witness" (maybe False ("minimal=101" `T.isInfixOf`) (vMessage v))
  , test "replay from the seed gives the identical verdict" $ do
      a <- check 42 (int 0 100000) off le100
      b <- check 42 (int 0 100000) off le100
      assertEq "same verdict" a b
  , test "False falsifies and only True holds" $ do
      v <- check 1 (int 0 5) off (\_ -> pure False)
      assertEq "id" (Just failureCounterexample) (vFailureId v)
  , test "zero cases is never a pass" $ do
      v <- checkPure 1 (int 0 5) off {optNumTests = 0} (const True)
      assertFalse "passed" (vPassed v)
      assertEq "id" (Just failureExhausted) (vFailureId v)
  , test "a throwing property is a panic" $ do
      v <- check 1 (int 0 5) off (\_ -> liftIO (throwIO Boom))
      assertEq "id" (Just failurePanic) (vFailureId v)
      assertTrue "type in message" (maybe False ("raised at iteration 1" `T.isInfixOf`) (vMessage v))
      assertEq "seed locally" (Just 1) (vSeed v)
      r <- check 1 (int 0 5) on (\_ -> liftIO (throwIO Boom))
      assertEq "id redacted" (Just failurePanic) (vFailureId r)
      assertEq "no seed" Nothing (vSeed r)
      assertTrue "type name only" (maybe False ("Boom" `T.isInfixOf`) (vMessage r))
  , test "a raise during shrinking is not a smaller counterexample" $ do
      -- candidates in [101, 999] throw; the minimal reported must not be one of them
      let prop :: Integer -> PropM Bool
          prop x
            | x >= 101 && x < 1000 = liftIO (throwIO Boom)
            | otherwise = pure (x <= 100)
      v <- check 1 (int 1000 100000) off prop
      assertEq "id" (Just failureCounterexample) (vFailureId v)
      let minimal = maybe 0 (read . T.unpack) (vMinimal v) :: Integer
      assertTrue "a real counterexample" (minimal >= 1000)
  , test "a generator error is exhausted" $ do
      v <- check 1 (filterG 5 (const False) (int 0 9)) off (\_ -> pure True)
      assertEq "id" (Just failureExhausted) (vFailureId v)
      assertTrue "message" (maybe False ("filter precondition" `T.isInfixOf`) (vMessage v))
      w <- check 1 (customGen (\_ _ -> ioError (userError "gen boom")) Nothing :: Gen Integer) off (\_ -> pure True)
      assertEq "other generator errors too" (Just failureExhausted) (vFailureId w)
      assertTrue "message shown locally" (maybe False ("gen boom" `T.isInfixOf`) (vMessage w))
      x <- check 1 (customGen (\_ _ -> ioError (userError "gen boom")) Nothing :: Gen Integer) on (\_ -> pure True)
      assertFalse "message hidden in CI" (maybe True ("gen boom" `T.isInfixOf`) (vMessage x))
  , test "a lying shrinker cannot change the witness" $ do
      let liar _ _ _ _ = pure (5 :: Integer, 3) -- 5 satisfies the property
      v <- checkWith Nothing (Just liar) 42 (int 0 100000) off le100
      assertEq "original witness" (vCounterexample v) (vMinimal v)
      assertEq "zero steps" 0 (vShrinkSteps v)
  , test "shrinker failures fall back to the original" $ do
      let boom _ _ _ _ = ioError (userError "shrinker boom") :: IO (Integer, Int)
          bad _ _ _ _ = pure (500, 99999) :: IO (Integer, Int)
          neg _ _ _ _ = pure (500, -1) :: IO (Integer, Int)
      mapM_
        ( \(label, s) -> do
            v <- checkWith Nothing (Just s) 42 (int 0 100000) off le100
            assertEq (label ++ " keeps the original") (vCounterexample v) (vMinimal v)
            assertEq (label ++ " steps") 0 (vShrinkSteps v)
        )
        [("throwing", boom), ("steps above the budget", bad), ("negative steps", neg)]
  , test "a shrinker value the property throws on is not adopted" $ do
      let prop x = if x == 500 then liftIO (throwIO Boom) else pure (x <= 100)
          s _ _ _ _ = pure (500, 4) :: IO (Integer, Int)
      v <- checkWith Nothing (Just s) 42 (int 600 100000) off prop
      assertEq "original" (vCounterexample v) (vMinimal v)
      assertEq "steps" 0 (vShrinkSteps v)
  , test "a good custom shrinker is used" $ do
      let s _ _ _ _ = pure (101, 5) :: IO (Integer, Int)
      v <- checkWith Nothing (Just s) 42 (int 0 100000) off le100
      assertEq "minimal" (Just "101") (vMinimal v)
      assertEq "steps" 5 (vShrinkSteps v)
  , test "CI on and unset redact everything" $
      mapM_
        ( \(label, o) -> do
            v <- check 42 (int 0 100000) o le100
            assertEq (label ++ " id") (Just failureCounterexample) (vFailureId v)
            assertTrue (label ++ " seed") (isNothing (vSeed v))
            assertTrue (label ++ " counterexample") (isNothing (vCounterexample v))
            assertTrue (label ++ " minimal") (isNothing (vMinimal v))
            assertTrue (label ++ " digest") (isJust (vDigest v))
            let msg = maybe "" id (vMessage v)
            assertFalse (label ++ " leaks") (any (`T.isInfixOf` msg) ["initial=", "minimal=", "seed="])
        )
        [("on", on), ("unset", unset)]
  , test "the default options redact" $ do
      v <- check 42 (int 0 100000) defaultOptions le100
      assertTrue "redacted" (isNothing (vMinimal v) && isNothing (vSeed v))
      assertEq "default is unset" CiUnset (optCi defaultOptions)
  , test "the digest is stable and witness sensitive" $ do
      a <- check 42 (int 0 100000) on le100
      b <- check 42 (int 0 100000) on le100
      assertEq "stable" (vDigest a) (vDigest b)
      assertTrue "16 lowercase hex digits" (maybe False (\d -> T.length d == 16 && T.all (`elem` ("0123456789abcdef" :: String)) d) (vDigest a))
      c <- check 42 (int 0 100000) on (pureProp (<= 200))
      assertTrue "different witness, different digest" (vDigest a /= vDigest c)
  , test "ciFromLookup" $ do
      assertEq "unset variable" CiOff (ciFromLookup Nothing)
      mapM_ (\s -> assertEq ("off " ++ show s) CiOff (ciFromLookup (Just s))) ["", "0", "false", "FALSE", " False ", "  "]
      mapM_ (\s -> assertEq ("on " ++ show s) CiOn (ciFromLookup (Just s))) ["1", "true", "yes", "anything"]
  , test "invalid options throw instead of producing a verdict" $ do
      assertThrows "negative tests" isArg (checkPure 1 u8 off {optNumTests = -1} (const True))
      assertThrows "too many tests" isArg (checkPure 1 u8 off {optNumTests = maxCases + 1} (const True))
      assertThrows "negative steps" isArg (checkPure 1 u8 off {optMaxShrinkSteps = -1} (const True))
      assertThrows "too many steps" isArg (checkPure 1 u8 off {optMaxShrinkSteps = 10001} (const True))
  , test "extreme seeds are fine" $ do
      a <- checkPure maxBound (int 0 9) off (<= 9)
      b <- checkPure 0 (int 0 9) off (<= 9)
      assertTrue "both pass" (vPassed a && vPassed b)
  , test "failure ids are closed" $ do
      assertEq "four ids" 4 (length failureIds)
      assertTrue "registered" (all isFailureId failureIds)
      assertFalse "unregistered" (isFailureId "unregistered-id")
      assertFalse "case sensitive" (isFailureId "Property-Panic")
      assertFalse "empty" (isFailureId "")
  , test "rendering a hostile witness still yields a verdict" $ do
      let g = customGen (\_ _ -> pure (Hostile 3)) Nothing
      v <- check 1 g off (\_ -> pure False)
      assertEq "id" (Just failureCounterexample) (vFailureId v)
      assertTrue "unrenderable" (maybe False ("<unrenderable>" `T.isInfixOf`) (vMessage v))
      r <- check 1 g on (\_ -> pure False)
      assertEq "redacted too" (Just failureCounterexample) (vFailureId r)
  , test "a message cannot forge a log line" $ do
      let g = customGen (\_ _ -> pure ("line1\nFAKE: ok\r\x202e" :: Text)) Nothing
      v <- check 1 g off (\_ -> pure False)
      let msg = maybe "" id (vMessage v)
      assertFalse "no raw newline" ("\n" `T.isInfixOf` msg || "\r" `T.isInfixOf` msg)
      assertFalse "no raw bidi override" ("\x202e" `T.isInfixOf` msg)
      assertTrue "escaped instead" ("\\n" `T.isInfixOf` msg)
      w <- check 1 (int 0 3) off (\_ -> liftIO (throwIO (userError "a\nb")))
      assertFalse "exception text escaped" ("\n" `T.isInfixOf` maybe "" id (vMessage w))
  , test "each case draws from the stream in order" $ do
      seen <- newIORef []
      _ <- check 7 (int 0 1000) off (\x -> liftIO (modifyIORef seen (x :)) >> pure True)
      xs <- reverse <$> readIORef seen
      rng <- newRng 7
      ys <- mapM (const (sample Nothing (int 0 1000) rng)) [1 .. 100 :: Int]
      assertEq "same candidates" ys xs
  , test "a passing check evaluates exactly num_tests cases" $ do
      n <- newIORef (0 :: Int)
      v <- check 1 u8 off {optNumTests = 37} (\_ -> liftIO (modifyIORef' n (+ 1)) >> pure True)
      c <- readIORef n
      assertEq "evaluations" 37 c
      assertEq "cases run" 37 (vCasesRun v)
  , test "the unused PropM plumbing behaves" $ do
      v <- check 1 u8 off {optNumTests = 3} (\_ -> do b <- askBudget; liftIO (isClosed b) >>= \c -> pure (not c))
      assertTrue "the scope is open while the property runs" (vPassed v)
      x <- withBudget' 
      assertTrue "applicative" x
  ]
  where
    withBudget' = do
      r <- newIORef False
      _ <- check 1 u8 off {optNumTests = 1} (\_ -> (,) <$> askBudget <*> pure () >>= \(b, ()) -> liftIO (writeIORef r (budgetDepth b == 0)) >> pure True)
      readIORef r
