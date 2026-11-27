-- | The two hazards that exist only in Haskell (VER05n 4): laziness and
-- asynchronous exceptions, plus the arithmetic and hostile-value hazards.
module HazardSpec (tests) where

import Control.Concurrent
import Control.DeepSeq (NFData (..))
import Control.Exception
import Control.Monad (forever)
import Data.IORef
import Data.Maybe (isJust)
import qualified Data.Text as T
import System.Timeout (timeout)

import CodingAdventures.TestBootstrap
import CodingAdventures.TestBootstrap.CorpusJson.Canon (sameVal)
import Harness

off :: Options
off = defaultOptions {optCi = CiOff}

-- | An asynchronous exception of our own (anything under SomeAsyncException).
data MyAsync = MyAsync deriving (Show)

instance Exception MyAsync where
  toException = asyncExceptionToException
  fromException = asyncExceptionFromException

-- | A synchronous exception whose displayException is itself bottom.
data Nasty = Nasty deriving (Show)

instance Exception Nasty where
  displayException _ = error "displayException boom"

-- | A property that throws the given exception.
throwing :: Exception e => e -> Integer -> PropM Bool
throwing e _ = liftIO (throwIO e)

-- | The check must rethrow the exception rather than report a verdict.
propagates :: (Exception e, Show e) => String -> e -> (SomeException -> Bool) -> IO ()
propagates label e matches = do
  r <- try (check 1 (int 0 5) off (throwing e))
  case r of
    Left x | matches x -> pure ()
    Left x -> failTest (label ++ ": wrong exception " ++ show x)
    Right v -> failTest (label ++ ": swallowed, verdict " ++ show (vFailureId v))

isThreadKilled, isUserInterrupt, isStackOverflow, isHeapOverflow, isMyAsync :: SomeException -> Bool
isThreadKilled e = fromException e == Just ThreadKilled
isUserInterrupt e = fromException e == Just UserInterrupt
isStackOverflow e = fromException e == Just StackOverflow
isHeapOverflow e = fromException e == Just HeapOverflow
isMyAsync e = case fromException e of Just MyAsync -> True; Nothing -> False

tests :: [Test]
tests =
  [ -- ------------------------------------------------------------------
    -- Hazard 1: laziness
    test "a lazy error in the property's Bool is a panic, not an engine crash" $ do
      v <- check 1 (int 0 5) off (\_ -> pure (error "lazy bool boom"))
      assertEq "id" (Just failurePanic) (vFailureId v)
      assertTrue "message" (maybe False ("lazy bool boom" `T.isInfixOf`) (vMessage v))
  , test "a bottom PropM action is a panic" $ do
      v <- check 1 (int 0 5) off (\_ -> error "bottom action")
      assertEq "id" (Just failurePanic) (vFailureId v)
  , test "a lazy error hidden inside a generated value is a generator error" $ do
      let g = customGen (\_ _ -> pure [1, error "lazy element" :: Integer]) Nothing
      v <- check 1 g off (\_ -> pure True)
      assertEq "exhausted" (Just failureExhausted) (vFailureId v)
      assertFalse "never a pass" (vPassed v)
  , test "a generator that is itself bottom is a generator error" $ do
      v <- check 1 (error "bottom gen" :: Gen Integer) off (\_ -> pure True)
      assertEq "exhausted" (Just failureExhausted) (vFailureId v)
      let g = customGen (\_ _ -> error "bottom sampler") Nothing :: Gen Integer
      w <- check 1 g off (\_ -> pure True)
      assertEq "exhausted" (Just failureExhausted) (vFailureId w)
  , test "a shrinker returning a lazy bottom keeps the original witness" $ do
      let lazyValue _ _ _ _ = pure (error "lazy shrunk" :: Integer, 3)
          lazySteps _ _ _ _ = pure (5 :: Integer, error "lazy steps" :: Int)
      mapM_
        ( \(label, s) -> do
            v <- checkWith Nothing (Just s) 42 (int 0 100000) off (pureProp (<= 100))
            assertEq (label ++ ": a counterexample") (Just failureCounterexample) (vFailureId v)
            assertEq (label ++ ": original") (vCounterexample v) (vMinimal v)
            assertEq (label ++ ": steps") 0 (vShrinkSteps v)
        )
        [("value", lazyValue), ("steps", lazySteps)]
  , test "a bottom property result during shrinking is not a smaller counterexample" $ do
      let prop :: Integer -> PropM Bool
          prop x = if x < 1000 && x > 100 then pure (error "lazy during shrink") else pure (x <= 100)
      v <- check 1 (int 1000 100000) off prop
      let minimal = maybe 0 (read . T.unpack) (vMinimal v) :: Integer
      assertTrue "real counterexample" (minimal >= 1000)
  , test "a lazily failing Canonical instance cannot break a verdict" $ do
      let g = customGen (\_ _ -> pure (Odd 1)) Nothing
      v <- check 1 g off (\_ -> pure False)
      assertEq "id" (Just failureCounterexample) (vFailureId v)
      assertTrue "unrenderable" (maybe False ("<unrenderable>" `T.isInfixOf`) (vMessage v))
  , test "the verdict fields are strict: reading them never throws" $ do
      v <- check 1 (int 0 5) off (\_ -> pure (error "x"))
      r <- try (evaluate (length (show v)))
      case r of
        Left (e :: SomeException) -> failTest ("verdict field threw: " ++ show e)
        Right _ -> pure ()
  , -- ------------------------------------------------------------------
    -- Hazard 2: asynchronous exceptions
    test "ThreadKilled, UserInterrupt, StackOverflow and HeapOverflow thrown by the property propagate" $ do
      propagates "ThreadKilled" ThreadKilled isThreadKilled
      propagates "UserInterrupt" UserInterrupt isUserInterrupt
      propagates "StackOverflow" StackOverflow isStackOverflow
      propagates "HeapOverflow" HeapOverflow isHeapOverflow
  , test "an application-defined asynchronous exception propagates" $
      propagates "MyAsync" MyAsync isMyAsync
  , test "a synchronous ErrorCall, ArithException and custom exception are property-panic" $ do
      mapM_
        ( \(label, act) -> do
            v <- check 1 (int 0 5) off (\_ -> liftIO act)
            assertEq label (Just failurePanic) (vFailureId v)
        )
        [ ("ErrorCall", throwIO (ErrorCall "plain"))
        , ("ArithException", throwIO DivideByZero)
        , ("IOException", ioError (userError "io"))
        , ("pure divide", evaluate (1 `div` (0 :: Int)) >> pure True)
        , ("custom", throwIO Nasty)
        ]
  , test "classification is by type, never by message text" $ do
      v <- check 1 (int 0 5) off (\_ -> liftIO (throwIO (ErrorCall "ThreadKilled")))
      assertEq "an ErrorCall mentioning ThreadKilled is an ordinary panic" (Just failurePanic) (vFailureId v)
      w <- check 1 (int 0 5) off (\_ -> liftIO (throwIO (ErrorCall "user interrupt")))
      assertEq "likewise" (Just failurePanic) (vFailureId w)
  , test "an exception whose displayException is bottom still yields a verdict" $ do
      v <- check 1 (int 0 5) off (throwing Nasty)
      assertEq "id" (Just failurePanic) (vFailureId v)
      assertTrue "unprintable" (maybe False ("<unprintable>" `T.isInfixOf`) (vMessage v))
  , test "an asynchronous exception from the generator propagates" $ do
      let g = customGen (\_ _ -> throwIO ThreadKilled) Nothing :: Gen Integer
      r <- try (check 1 g off (\_ -> pure True))
      case r of
        Left e | isThreadKilled e -> pure ()
        Left e -> failTest ("wrong exception " ++ show e)
        Right _ -> failTest "swallowed"
  , test "an asynchronous exception during shrinking propagates" $ do
      let prop x = if x < 50 then liftIO (throwIO ThreadKilled) else pure (x <= 10)
          probe _ _ f _ = f 3 >> pure (3 :: Integer, 1)
      r <- try (checkWith Nothing (Just probe) 1 (int 100 1000) off prop)
      case r of
        Left e | isThreadKilled e -> pure ()
        Left e -> failTest ("wrong exception " ++ show e)
        Right v -> failTest ("swallowed: " ++ show (vFailureId v))
  , test "timeout cancels a property that never returns" $ do
      r1 <- timeout 300000 (check 1 (int 0 5) off (\_ -> liftIO (forever (threadDelay 1000))))
      assertTrue "blocked property cancelled" (r1 == Nothing)
      r2 <- timeout 300000 (check 1 (int 0 5) off (\_ -> pure (length [1 :: Integer ..] > 0)))
      assertTrue "CPU-bound property cancelled" (r2 == Nothing)
  , test "timeout interrupts a loop that never allocates (-fno-omit-yields)" $ do
      r <- timeout 500000 (check 1 (int 0 5) off (\_ -> pure (spin 0)))
      assertTrue "interrupted, not hung" (r == Nothing)
  , test "timeout cancels a property that loops during shrinking" $ do
      let prop x = if x > 100 then pure False else pure (length [1 :: Integer ..] > 0 && x > 1000)
      r <- timeout 500000 (check 1 (int 200 1000) off prop)
      assertTrue "cancelled" (r == Nothing || maybe False (\v -> vFailureId v == Just failureCounterexample) r)
  , test "killThread of a thread inside check leaves the scope closed" $ do
      scopeRef <- newIORef Nothing
      started <- newEmptyMVar
      finished <- newEmptyMVar
      t <-
        forkIO $
          ( check 1 (int 0 5) off (\_ -> do b <- askBudget; liftIO (writeIORef scopeRef (Just b) >> putMVar started () >> threadDelay 60000000); pure True)
              >> pure ()
          )
            `finally` putMVar finished ()
      takeMVar started
      killThread t
      takeMVar finished
      Just b <- readIORef scopeRef
      closed <- isClosed b
      assertTrue "scope closed" closed
  , test "the budget scope is closed when the property throws an asynchronous exception" $ do
      scopeRef <- newIORef Nothing
      _ <- try (check 1 (int 0 5) off (\_ -> do b <- askBudget; liftIO (writeIORef scopeRef (Just b)); liftIO (throwIO ThreadKilled)))
        :: IO (Either SomeException Verdict)
      Just b <- readIORef scopeRef
      assertTrue "closed" =<< isClosed b
  , -- ------------------------------------------------------------------
    -- Hazard 4: arithmetic and Integer input
    test "Word64 arithmetic wraps natively" $ do
      assertEq "wrap" 0 (maxBound + 1 :: Word)
      r <- newRng maxBound
      w <- nextU64 r
      r2 <- newRng (maxBound + 1)
      w2 <- nextU64 r2
      assertTrue "the seed wrapped without error" (w /= w2)
  , test "an Integer span wider than the cap is refused before any draw" $ do
      r <- newRng 1
      before <- rngState r
      res <- try (sample Nothing (int 0 (2 ^ (50000 :: Int))) r)
      case res of
        Left (ArgumentError _) -> pure ()
        Left e -> failTest ("wrong error " ++ show e)
        Right _ -> failTest "expected a refusal"
      after <- rngState r
      assertEq "no word consumed" before after
  , -- ------------------------------------------------------------------
    -- Hazard 5: hostile values stay bounded
    test "rendering and equality stay bounded on enormous and infinite values" $ do
      out <- within 30 (render (VList (map VInt [1 ..])))
      assertTrue "infinite list rendered and cut" ("..." `T.isSuffixOf` out)
      big <- within 30 (render (VList (replicate 1000000 (VText "x"))))
      assertTrue "million nodes cut" ("..." `T.isSuffixOf` big)
      txt <- within 30 (render (VText (T.replicate (10 * 1048576) "a")))
      assertTrue "ten MiB text cut" (T.length txt < 1048576 + 16)
      assertFalse "equality on infinite lists answers False" (sameVal (VList (map VInt [1 ..])) (VList (map VInt [1 ..])))
  , test "a witness that is an infinite list inside a Canonical wrapper cannot hang the verdict" $ do
      let g = customGen (\_ _ -> pure (Endless 0)) Nothing
      v <- within 30 (check 1 g off (\_ -> pure False))
      assertEq "id" (Just failureCounterexample) (vFailureId v)
      assertTrue "message bounded" (maybe False ((< 2100000) . T.length) (vMessage v))
  , -- ------------------------------------------------------------------
    -- Hazard 6: nothing hangs
    test "within turns a hang into a failure" $ do
      r <- try (within 1 (threadDelay 5000000))
      case r of
        Left (AssertionFailed _) -> pure ()
        Right () -> failTest "within did not time out"
  , test "a property with isJust plumbing" $ do
      v <- check 1 (int 0 5) off (\x -> pure (isJust (Just x)))
      assertTrue "passes" (vPassed v)
  ]

-- | A witness whose toVal raises when it is looked at.
newtype Odd = Odd Int

instance Canonical Odd where
  toVal (Odd _) = VList [VInt 1, error "odd toVal"]

instance NFData Odd where rnf (Odd n) = rnf n

-- | A witness that is an infinite structure behind a finite NFData.
newtype Endless = Endless Int

instance NFData Endless where rnf (Endless n) = rnf n

instance Canonical Endless where
  toVal (Endless n) = VList (map VInt [toInteger n ..])

-- | A tight non-allocating loop GHC cannot optimise away or inline.
spin :: Int -> Bool
spin n = spin n
{-# NOINLINE spin #-}
