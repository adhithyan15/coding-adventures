-- | A tiny test harness (no Hackage): named IO actions, each run under a
-- 60-second timeout so a test can FAIL but never hang (VER05n 10, hazard 6).
module Harness
  ( Test
  , test
  , runTests
  , assertEq
  , assertTrue
  , assertFalse
  , assertLeft
  , assertThrows
  , assertThrowsAny
  , failTest
  , within
  ) where

import Control.Exception
import Control.Monad (unless)
import Data.IORef
import System.Exit (exitFailure)
import System.IO (hFlush, stdout)
import System.Timeout (timeout)

data Test = Test String (IO ())

test :: String -> IO () -> Test
test = Test

failTest :: String -> IO a
failTest msg = throwIO (AssertionFailed msg)

assertEq :: (Eq a, Show a) => String -> a -> a -> IO ()
assertEq label expected actual =
  unless (expected == actual) (failTest (label ++ ": expected " ++ show expected ++ ", got " ++ show actual))

assertTrue :: String -> Bool -> IO ()
assertTrue label b = unless b (failTest (label ++ ": expected True"))

assertFalse :: String -> Bool -> IO ()
assertFalse label b = unless (not b) (failTest (label ++ ": expected False"))

assertLeft :: String -> Either a b -> IO ()
assertLeft label = \case
  Left _ -> pure ()
  Right _ -> failTest (label ++ ": expected Left, got Right")

-- | The action must throw an exception of type @e@ satisfying the predicate.
assertThrows :: forall e a. Exception e => String -> (e -> Bool) -> IO a -> IO ()
assertThrows label ok act = do
  r <- try act
  case r of
    Left e | ok e -> pure ()
    Left e -> failTest (label ++ ": wrong exception " ++ show e)
    Right _ -> failTest (label ++ ": expected an exception")

assertThrowsAny :: String -> IO a -> IO ()
assertThrowsAny label act = assertThrows label (\(_ :: SomeException) -> True) act

-- | Runs the action under a timeout of @seconds@; fails (never hangs) on expiry.
within :: Int -> IO a -> IO a
within seconds act = do
  r <- timeout (seconds * 1000000) act
  maybe (failTest ("timed out after " ++ show seconds ++ "s")) pure r

runTests :: String -> [Test] -> IO ()
runTests suite tests = do
  failures <- newIORef (0 :: Int)
  mapM_ (run failures) tests
  n <- readIORef failures
  putStrLn (suite ++ ": " ++ show (length tests) ++ " tests, " ++ show n ++ " failed")
  hFlush stdout
  unless (n == 0) exitFailure
  where
    run failures (Test name act) = do
      r <- timeout (60 * 1000000) (try act)
      case r of
        Just (Right ()) -> pure ()
        Just (Left (e :: SomeException)) -> bad failures name (show e)
        Nothing -> bad failures name "timed out after 60s"
    bad failures name msg = do
      modifyIORef' failures (+ 1)
      putStrLn ("FAIL " ++ name ++ ": " ++ take 600 msg)
      hFlush stdout
