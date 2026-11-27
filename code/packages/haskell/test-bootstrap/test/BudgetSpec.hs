-- | The hierarchical budget tree (VER05e 4.1 / VER05n 5): the required set of
-- VER05e's @ShrinkBudgetTest@ plus the Haskell-specific handle, thread and
-- spent-handle cases.
module BudgetSpec (tests) where

import Control.Concurrent
import Control.Exception
import Control.Monad (forM_, replicateM, replicateM_, void)
import Data.IORef
import qualified Data.Text as T

import CodingAdventures.TestBootstrap
import Harness

off :: Options
off = defaultOptions {optCi = CiOff}

-- | Runs an action under a fresh root and returns the handle (closed) with the result.
rooted :: Int -> (Budget -> IO a) -> IO (Budget, a)
rooted cap act = do
  ref <- newIORef Nothing
  r <- withRoot defaultConfig cap (\b -> writeIORef ref (Just b) >> act b)
  Just b <- readIORef ref
  pure (b, r)

passed :: Verdict -> Bool
passed = vPassed

tests :: [Test]
tests =
  [ test "attempts are capped and a refusal charges nothing" $ do
      (b, results) <- rooted 5 (\b -> replicateM 7 (charge b 1))
      assertEq "five accepted then refused" (replicate 5 True ++ [False, False]) results
      a <- attempts b
      assertEq "attempts" 5 a
      w <- work b
      assertEq "work" 5 w
      assertTrue "refused flag" =<< isRefused b
  , test "room reports whether one more attempt fits" $ do
      _ <- rooted 2 $ \b -> do
        r1 <- room b
        _ <- charge b 1
        _ <- charge b 1
        r2 <- room b
        assertTrue "room before" r1
        assertFalse "no room after" r2
      pure ()
  , test "a refused take leaves every counter unchanged along the whole path" $ do
      _ <- rooted 100 $ \root ->
        withChild defaultConfig (Just root) 0 $ \child -> do
          -- the child's local cap is 16 * 0 + 64 = 64; spend 64 on it
          replicateM_ 64 (charge child 1)
          ca <- attempts child
          ra <- attempts root
          cw <- work child
          rw <- work root
          ok <- charge child 1
          assertFalse "refused" ok
          ca' <- attempts child
          ra' <- attempts root
          cw' <- work child
          rw' <- work root
          assertEq "child attempts" ca ca'
          assertEq "root attempts" ra ra'
          assertEq "child work" cw cw'
          assertEq "root work" rw rw'
      pure ()
  , test "a refusal at the root leaves the child counters unchanged too" $ do
      _ <- rooted 3 $ \root ->
        withChild defaultConfig (Just root) 1000 $ \child -> do
          replicateM_ 3 (charge child 1)
          ca <- attempts child
          ok <- charge child 1
          ca' <- attempts child
          assertFalse "refused by the root" ok
          assertEq "child unchanged" ca ca'
          assertEq "child shows 3" 3 ca'
      pure ()
  , test "a huge cost is refused without wrapping" $ do
      _ <- rooted 100 $ \b -> do
        r1 <- charge b maxBound
        r2 <- chargeCopy b maxBound
        r3 <- charge b (maxBound - 1)
        assertFalse "maxBound" r1
        assertFalse "maxBound copy" r2
        assertFalse "maxBound - 1" r3
        w <- work b
        a <- attempts b
        assertEq "no work" 0 w
        assertEq "no attempts" 0 a
        ok <- charge b 5
        assertTrue "a normal charge still works" ok
      pure ()
  , test "a zero or negative cost is charged as one" $ do
      (b, _) <- rooted 10 $ \x -> charge x 0 >> charge x (-5)
      w <- work b
      assertEq "two units of work" 2 w
  , test "the attempt cap is enforced separately from work" $ do
      _ <- rooted 3 $ \b -> do
        replicateM_ 3 (charge b 1)
        copy <- chargeCopy b 1 -- work only, but the attempts are spent
        assertFalse "work-only charge refused once attempts are spent" copy
      (b2, _) <- rooted 3 $ \b -> replicateM_ 50 (chargeCopy b 1)
      a <- attempts b2
      assertEq "work-only charges never consume attempts" 0 a
  , test "the work cap" $ do
      let cfg = defaultConfig {cfgMaxWork = 10}
      withRoot cfg 100 $ \b -> do
        r1 <- charge b 6
        r2 <- charge b 5
        r3 <- charge b 4
        assertEq "6, 5 refused, 4" [True, False, True] [r1, r2, r3]
        w <- work b
        assertEq "work" 10 w
  , test "chargedFalsifies stops at the cap and does not run the property" $ do
      calls <- newIORef (0 :: Int)
      _ <- rooted 3 $ \b -> do
        let f = chargedFalsifies b (\_ -> modifyIORef' calls (+ 1) >> pure True)
        rs <- mapM f [1 .. 6 :: Int]
        assertEq "three then False" [True, True, True, False, False, False] rs
      n <- readIORef calls
      assertEq "property calls" 3 n
  , test "a closed handle is spent: it refuses every charge and is never a fresh root" $ do
      (h, _) <- rooted 10 (const (pure ()))
      closed <- isClosed h
      assertTrue "closed" closed
      r1 <- charge h 1
      r2 <- chargeCopy h 1
      rm <- room h
      assertEq "refused" [False, False, False] [r1, r2, rm]
      -- a child of a closed handle is spent too (no resolution to an open ancestor)
      c <- withChild defaultConfig (Just h) 10 (\child -> charge child 1)
      assertFalse "child of a closed handle" c
      -- a shrinker under a stale handle is starved without running the property
      calls <- newIORef (0 :: Int)
      r <- shrinkIntWith defaultConfig (Just h) 500 0 0 1000 1000 (\_ -> modifyIORef' calls (+ 1) >> pure False)
      n <- readIORef calls
      assertEq "starved" (500, 0) r
      assertEq "property not run" 0 n
  , test "a stale handle used for a check is exhausted, never a pass" $ do
      (h, _) <- rooted 100 (const (pure ()))
      calls <- newIORef (0 :: Int)
      v <- withBudget h (checkM 1 (int 0 9) off (\_ -> liftIO (modifyIORef' calls (+ 1)) >> pure True))
      n <- readIORef calls
      assertFalse "passed" (passed v)
      assertEq "exhausted" (Just failureExhausted) (vFailureId v)
      assertEq "nothing evaluated" 0 n
  , test "a child scope closes with its body, an exception included" $ do
      ref <- newIORef Nothing
      r <- try (withRoot defaultConfig 5 (\b -> writeIORef ref (Just b) >> throwIO (userError "scope body failed")))
      case r of
        Left (_ :: IOException) -> pure ()
        Right () -> failTest "expected the exception"
      Just b <- readIORef ref
      assertTrue "closed after the exception" =<< isClosed b
      ref2 <- newIORef Nothing
      withRoot defaultConfig 5 $ \root -> do
        r2 <- try (withChild defaultConfig (Just root) 10 (\c -> writeIORef ref2 (Just c) >> throwIO (userError "inner")))
        case r2 of
          Left (_ :: IOException) -> pure ()
          Right () -> failTest "expected the exception"
        open <- isClosed root
        assertFalse "the outer scope stays open" open
      Just c <- readIORef ref2
      assertTrue "inner closed" =<< isClosed c
  , test "a scope is closed when the thread running it is killed" $ do
      ref <- newIORef Nothing
      started <- newEmptyMVar
      done <- newEmptyMVar
      t <- forkIO ((withRoot defaultConfig 5 (\b -> writeIORef ref (Just b) >> putMVar started () >> threadDelay 60000000)) `finally` putMVar done ())
      takeMVar started
      killThread t
      takeMVar done
      Just b <- readIORef ref
      assertTrue "closed after ThreadKilled" =<< isClosed b
  , test "depth cap: deeper scopes join instead of nesting" $ do
      maxSeen <- newIORef (0 :: Int)
      let go :: Maybe Budget -> Int -> IO ()
          go parent n
            | n == 0 = pure ()
            | otherwise = withChild defaultConfig parent 10 $ \b -> do
                modifyIORef' maxSeen (max (budgetDepth b))
                go (Just b) (n - 1)
      go Nothing 200
      d <- readIORef maxSeen
      assertEq "depth stops at 63" 63 d
  , test "check nesting is capped: a recursive property can never pass" $ do
      deepest <- newIORef (0 :: Int)
      let o = off {optNumTests = 1, optMaxShrinkSteps = 0}
          level :: Int -> PropM Bool
          level n = do
            liftIO (modifyIORef' deepest (max n))
            v <- checkM 1 (int 0 0) o (\_ -> level (n + 1))
            pure (passed v)
      v <- check 1 (int 0 0) o (\_ -> level 1)
      d <- readIORef deepest
      assertFalse "never a pass" (passed v)
      assertTrue "stopped well below the stack" (d <= 64)
  , test "nested checks to depth 3 stay inside the root cap" $ do
      evals <- newIORef (0 :: Int)
      rootRef <- newIORef Nothing
      let o n = off {optNumTests = n, optMaxShrinkSteps = 1}
          leaf :: Integer -> PropM Bool
          leaf _ = liftIO (modifyIORef' evals (+ 1)) >> pure True
          mid :: Integer -> PropM Bool
          mid _ = checkM 2 (int 0 9) (o 50) leaf >>= \v -> pure (passed v || True)
          top :: Integer -> PropM Bool
          top _ = do
            askBudget >>= \b -> liftIO (writeIORef rootRef (Just b))
            checkM 3 (int 0 9) (o 50) mid >>= \v -> pure (passed v || True)
      v <- check 1 (int 0 9) (o 2) top
      n <- readIORef evals
      Just b <- readIORef rootRef
      a <- attempts b
      assertTrue "bounded by the root cap" (a <= maxAttemptsOf b)
      assertTrue ("evaluations " ++ show n ++ " stay far below 2*50*50*50") (n <= maxAttemptsOf b)
      assertFalse "starvation is never a pass" (passed v)
      assertEq "exhausted" (Just failureExhausted) (vFailureId v)
  , test "branching: many checks per evaluation stay inside the root cap" $ do
      evals <- newIORef (0 :: Int)
      rootRef <- newIORef Nothing
      let o = off {optNumTests = 30, optMaxShrinkSteps = 1}
          inner :: Integer -> PropM Bool
          inner _ = liftIO (modifyIORef' evals (+ 1)) >> pure True
          top :: Integer -> PropM Bool
          top _ = do
            askBudget >>= \b -> liftIO (writeIORef rootRef (Just b))
            forM_ [1 .. 10 :: Word] (\i -> checkM (fromIntegral i) (int 0 9) o inner)
            pure True
      v <- check 1 (int 0 9) o {optNumTests = 3} top
      n <- readIORef evals
      Just b <- readIORef rootRef
      assertTrue "evaluations within the cap" (n <= maxAttemptsOf b)
      assertFalse "not a pass when starved" (passed v && n >= maxAttemptsOf b)
  , test "a recursing custom shrinker stays inside the root cap" $ do
      evals <- newIORef (0 :: Int)
      rootRef <- newIORef Nothing
      let g = int 0 1000000
          -- a shrinker that calls the public shrinker again, five levels deep
          recur :: Int -> Shrinker Integer
          recur 0 _ v _ _ = pure (v, 0)
          recur d b v f n = do
            (v', s) <- shrinkGen defaultConfig (Just b) g v f n
            (v'', s') <- recur (d - 1) b v' f n
            pure (v'', s + s')
          prop :: Integer -> PropM Bool
          prop x = do
            askBudget >>= \b -> liftIO (writeIORef rootRef (Just b))
            liftIO (modifyIORef' evals (+ 1))
            pure (x <= 100)
      v <- checkWith Nothing (Just (recur 5)) 42 g off prop
      n <- readIORef evals
      Just b <- readIORef rootRef
      assertTrue "outcome is a verdict" (not (passed v))
      assertTrue ("total evaluations " ++ show n ++ " within the root cap") (n <= 100 + maxAttemptsOf b)
  , test "a filter predicate is not invoked under a spent budget" $ do
      calls <- newIORef (0 :: Int)
      let g = filterM 100 (\_ -> modifyIORef' calls (+ 1) >> pure True) (int 0 1000)
      _ <- rooted 5 $ \b -> do
        -- spend the root first
        replicateM_ 5 (charge b 1)
        r <- shrinkGen defaultConfig (Just b) g 500 (\_ -> pure True) 1000
        assertEq "starved" (500, 0) r
      n <- readIORef calls
      assertEq "predicate never ran" 0 n
  , test "a filter sampler rejecting everything stays inside the cap in a nested check" $ do
      calls <- newIORef (0 :: Int)
      rootRef <- newIORef Nothing
      let o = off {optNumTests = 5, optMaxShrinkSteps = 0}
          rejecting = filterM 10000 (\_ -> modifyIORef' calls (+ 1) >> pure False) (int 0 9)
          top :: Integer -> PropM Bool
          top _ = do
            askBudget >>= \b -> liftIO (writeIORef rootRef (Just b))
            v <- checkM 1 rejecting o (\_ -> pure True)
            pure (passed v || True)
      v <- check 1 (int 0 9) o top
      n <- readIORef calls
      Just b <- readIORef rootRef
      assertTrue ("predicate calls " ++ show n ++ " within the cap") (n <= maxAttemptsOf b)
      assertFalse "never a pass" (passed v)
  , test "stacked filters cannot multiply retries at the top level" $ do
      calls <- newIORef (0 :: Int)
      let cfg = defaultConfig {cfgMaxWork = 4000}
          rej = filterM 10000 (\_ -> modifyIORef' calls (+ 1) >> pure False)
          g = rej (rej (rej (int 0 9)))
      v <- check 1 g off {optConfig = cfg} (\_ -> pure True)
      n <- readIORef calls
      assertEq "exhausted" (Just failureExhausted) (vFailureId v)
      assertTrue ("retries " ++ show n ++ " bounded by the work cap, not 10000^3") (n <= 4000)
  , test "top-level filter retries are work, not attempts" $ do
      rootRef <- newIORef Nothing
      let g = filterG 10000 (< 3) (int 0 999)
      v <- check 1 g off {optNumTests = 100} (\_ -> askBudget >>= \b -> liftIO (writeIORef rootRef (Just b)) >> pure True)
      Just b <- readIORef rootRef
      a <- attempts b
      w <- work b
      assertTrue "passed" (passed v)
      assertEq "no attempts were consumed by filtering" 0 a
      assertTrue "retries were charged as work" (w > 100)
  , test "a passing inner check with a huge num_tests is bounded under an outer scope" $ do
      calls <- newIORef (0 :: Int)
      let o = off {optNumTests = 1, optMaxShrinkSteps = 0}
          top :: Integer -> PropM Bool
          top _ = do
            v <- checkM 1 (int 0 9) off {optNumTests = 100000, optMaxShrinkSteps = 0} (\_ -> liftIO (modifyIORef' calls (+ 1)) >> pure True)
            pure (passed v || True)
      v <- check 1 (int 0 9) o top
      n <- readIORef calls
      assertTrue ("inner evaluations " ++ show n ++ " are far below 100000") (n < 200)
      assertFalse "starved is not a pass" (passed v)
  , test "shrinkInt and simulateReplay inside a property are charged to the root" $ do
      rootRef <- newIORef Nothing
      let o = off {optNumTests = 2, optMaxShrinkSteps = 0}
          top :: Integer -> PropM Bool
          top _ = do
            b <- askBudget
            liftIO (writeIORef rootRef (Just b))
            _ <- liftIO (shrinkIntWith defaultConfig (Just b) 54321 0 0 100000 1000 (pure . (<= 100)))
            _ <- liftIO (simulateReplayWith defaultConfig (Just b) 42 (pure . (<= 100)))
            pure True
      _ <- check 1 (int 0 9) o top
      Just b <- readIORef rootRef
      a <- attempts b
      assertTrue "charged to the root" (a > 15)
      assertTrue "within the cap" (a <= maxAttemptsOf b)
  , test "a starved nested check is exhausted but a top-level check still runs" $ do
      calls <- newIORef (0 :: Int)
      _ <- rooted 1 $ \b -> do
        _ <- charge b 1 -- spend the only attempt
        v <- withBudget b (checkM 1 (int 0 9) off (\_ -> liftIO (modifyIORef' calls (+ 1)) >> pure True))
        assertEq "nested: exhausted" (Just failureExhausted) (vFailureId v)
      n <- readIORef calls
      assertEq "nested evaluated nothing" 0 n
      v <- check 1 (int 0 9) off {optNumTests = 5} (\_ -> liftIO (modifyIORef' calls (+ 1)) >> pure True)
      m <- readIORef calls
      assertTrue "top level passes" (passed v)
      assertEq "top level evaluated 5" 5 m
  , test "a nested check pays one attempt at entry" $ do
      _ <- rooted 10 $ \b -> do
        before <- attempts b
        v <- withBudget b (checkM 1 (int 0 9) off {optNumTests = 1} (\_ -> pure True))
        after <- attempts b
        assertTrue "passed" (passed v)
        assertEq "one attempt" 1 (after - before)
      pure ()
  , test "a nested check inside the case loop charges the outer root" $ do
      rootRef <- newIORef Nothing
      let top :: Integer -> PropM Bool
          top _ = do
            b <- askBudget
            liftIO (writeIORef rootRef (Just b))
            v <- checkM 1 (int 0 9) off {optNumTests = 4} (\_ -> pure True)
            pure (passed v)
      v <- check 1 (int 0 9) off {optNumTests = 6} top
      Just b <- readIORef rootRef
      a <- attempts b
      assertTrue "passed" (passed v)
      -- 6 outer cases x (1 entry + 3 later cases) = 24 attempts on the shared root
      assertEq "charged" 24 a
  , test "a nested check sampling large values is charged as work" $ do
      let cfg = defaultConfig {cfgMaxWork = 2500}
      withRoot cfg 1000 $ \root -> do
        v <- checkWith (Just root) Nothing 1 (list u8 1000 1000) off {optNumTests = 10} (\_ -> pure True)
        w <- work root
        assertFalse "starved, never a pass" (passed v)
        assertTrue "work stayed under the cap" (w <= 2500)
  , test "a refused re-verification reports the original witness with zero steps" $ do
      withRoot defaultConfig 2 $ \root -> do
        let burn sc _ _ _ = charge sc 1 >> pure (101 :: Integer, 5)
        v <- checkWith (Just root) (Just burn) 42 (int 0 100000) off (pureProp (<= 100))
        assertEq "counterexample" (Just failureCounterexample) (vFailureId v)
        assertEq "zero steps" 0 (vShrinkSteps v)
        assertEq "the original witness" (vCounterexample v) (vMinimal v)
  , test "starvation does not override a falsified verdict" $ do
      let o = off {optNumTests = 1, optMaxShrinkSteps = 0}
          top :: Integer -> PropM Bool
          top _ = do
            _ <- checkM 1 (int 0 9) off {optNumTests = 200, optMaxShrinkSteps = 0} (\_ -> pure True)
            pure False
      v <- check 1 (int 0 9) o top
      assertEq "still a counterexample" (Just failureCounterexample) (vFailureId v)
  , test "a re-entrant check matches a standalone run while the outer budget has room" $ do
      standalone <- check 42 (int 0 100000) off (pureProp (<= 100))
      nested <- withRoot defaultConfig 100000 (\root -> checkWith (Just root) Nothing 42 (int 0 100000) off (pureProp (<= 100)))
      assertEq "same verdict" standalone nested
  , test "concurrent charging never exceeds the cap" $ do
      done <- newEmptyMVar
      oks <- newIORef (0 :: Int)
      (b, _) <- rooted 500 $ \b -> do
        forM_ [1 .. 8 :: Int] $ \_ ->
          void . forkIO $ do
            n <- length . filter id <$> replicateM 1000 (charge b 1)
            atomicModifyIORef' oks (\x -> (x + n, ()))
            putMVar done ()
        replicateM_ 8 (takeMVar done)
      total <- readIORef oks
      a <- attempts b
      assertEq "exactly the cap was granted" 500 total
      assertEq "attempts" 500 a
  , test "a thread carrying the handle charges the same tree" $ do
      done <- newEmptyMVar
      rootRef <- newIORef Nothing
      let top :: Integer -> PropM Bool
          top _ = do
            b <- askBudget
            liftIO (writeIORef rootRef (Just b))
            _ <- forkWithBudget $ do
              v <- checkM 1 (int 0 9) off {optNumTests = 3} (\_ -> pure True)
              liftIO (putMVar done (passed v))
            r <- liftIO (takeMVar done)
            pure r
      v <- check 1 (int 0 9) off {optNumTests = 1} top
      Just b <- readIORef rootRef
      a <- attempts b
      assertTrue "passed" (passed v)
      -- the forked check paid 1 entry + 2 later cases on the shared root
      assertEq "charged by the thread" 3 a
  , test "a thread started with a bare forkIO gets a fresh root (the documented escape)" $ do
      done <- newEmptyMVar
      rootRef <- newIORef Nothing
      let top :: Integer -> PropM Bool
          top _ = do
            b <- askBudget
            liftIO (writeIORef rootRef (Just b))
            _ <- liftIO . forkIO $ do
              v <- check 1 (int 0 9) off {optNumTests = 3} (\_ -> pure True)
              putMVar done (passed v)
            liftIO (takeMVar done)
      _ <- check 1 (int 0 9) off {optNumTests = 1} top
      Just b <- readIORef rootRef
      a <- attempts b
      assertEq "the outer root was not charged by the thread" 0 a
  , test "a thread that outlives its scope finds the handle spent" $ do
      hRef <- newIORef Nothing
      let top :: Integer -> PropM Bool
          top _ = askBudget >>= \b -> liftIO (writeIORef hRef (Just b)) >> pure True
      _ <- check 1 (int 0 9) off {optNumTests = 1} top
      Just b <- readIORef hRef
      done <- newEmptyMVar
      _ <- forkIO (withBudget b (checkM 1 (int 0 9) off (\_ -> pure True)) >>= putMVar done)
      v <- takeMVar done
      assertFalse "late thread cannot pass" (passed v)
      assertEq "exhausted" (Just failureExhausted) (vFailureId v)
    , test "forkWithBudget children run unmasked and report only the exception type" $ do
      maskRef <- newEmptyMVar
      errRef <- newEmptyMVar
      let top :: Integer -> PropM Bool
          top _ = do
            _ <- forkWithBudgetHandler (putMVar errRef . T.unpack) (liftIO (getMaskingState >>= putMVar maskRef) >> liftIO (throwIO (userError "SECRET text")))
            pure True
      v <- mask_ (check 1 (int 0 9) off {optNumTests = 1} top)
      assertTrue "passed" (passed v)
      st <- takeMVar maskRef
      assertEq "unmasked" Unmasked st
      e <- takeMVar errRef
      assertEq "type name only, no message" "IOException" e
      -- the default handler drops the exception silently
      done <- newEmptyMVar
      _ <- check 1 (int 0 9) off {optNumTests = 1} (\_ -> forkWithBudget (liftIO (putMVar done () >> throwIO (ErrorCall "x"))) >> pure True)
      takeMVar done
  ]
