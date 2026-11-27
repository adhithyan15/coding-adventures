-- | The honest runner (VER05 7.1, VER05e 5, VER05n 4 to 6).
--
-- * zero cases evaluated is @property-exhausted@, never a pass;
-- * a generator error is @property-exhausted@;
-- * a property that throws (or whose 'Bool' is a lazy bottom) is @property-panic@;
-- * the reported minimal witness is RE-VERIFIED: if the shrinker throws, returns
--   something that no longer fails, a value the property throws on, or a
--   nonsensical step count, the original candidate is reported with 0 steps;
-- * redacted mode never emits the witness or the seed;
-- * the call always returns a verdict (invalid options throw instead);
-- * asynchronous exceptions are NEVER swallowed.
--
-- /Laziness./  Every value the runner relies on is forced INSIDE the handler
-- that is supposed to catch its failure: a sampled candidate with @evaluate
-- (force v)@ in the sampling handler, the property's 'Bool' with @evaluate@ in
-- the property's handler, the shrinker's result in the shrink handler.  A lazy
-- @error@ hidden in a thunk would otherwise escape the @try@ and crash the
-- engine later.
--
-- There is no wall-clock timeout: @property-timeout@ is registered and never
-- produced.  A caller that needs one wraps 'check' in @System.Timeout.timeout@,
-- which works because asynchronous exceptions propagate.
module CodingAdventures.TestBootstrap.Pbt.Runner
  ( PropM
  , runPropM
  , liftIO
  , askBudget
  , pureProp
  , withBudget
  , forkWithBudget
  , forkWithBudgetHandler
  , check
  , checkPure
  , checkM
  , checkWith
  , valWork
  ) where

import Control.Concurrent (ThreadId, forkIOWithUnmask)
import Control.DeepSeq (NFData, force)
import Control.Exception (SomeException, catch, evaluate, fromException, try)
import Control.Monad.IO.Class (MonadIO (..))
import qualified Data.ByteString as B
import qualified Data.Text as T
import qualified Data.Text.Encoding as TE
import Data.Word (Word64)
import Numeric (showHex)

import CodingAdventures.TestBootstrap.CorpusJson.Canon (render)
import CodingAdventures.TestBootstrap.CorpusJson.Val
import CodingAdventures.TestBootstrap.Error (PbtError (ExhaustedError))
import CodingAdventures.TestBootstrap.Pbt.Budget
import CodingAdventures.TestBootstrap.Pbt.Gen
import CodingAdventures.TestBootstrap.Pbt.Safe
import CodingAdventures.TestBootstrap.Pbt.Types
import CodingAdventures.TestBootstrap.Prng (fnv1a64)
import Data.Text (Text)

-- | The monad a property runs in: a function from the budget scope the running
-- check opened to an 'IO' action (a hand-written @ReaderT Budget IO@, no @mtl@).
-- A property that nests a check, a shrinker or a generator reaches the outer
-- budget through 'askBudget' or 'checkM'.
newtype PropM a = PropM {unPropM :: Budget -> IO a}

runPropM :: PropM a -> Budget -> IO a
runPropM = unPropM

instance Functor PropM where
  fmap f (PropM g) = PropM (fmap f . g)

instance Applicative PropM where
  pure x = PropM (\_ -> pure x)
  PropM f <*> PropM x = PropM (\b -> f b <*> x b)

instance Monad PropM where
  PropM m >>= k = PropM (\b -> m b >>= \a -> unPropM (k a) b)

instance MonadIO PropM where
  liftIO io = PropM (const io)

-- | The scope the running check opened (the handle to pass to a thread, a
-- nested shrinker or a nested generator).
askBudget :: PropM Budget
askBudget = PropM pure

-- | Lifts a pure predicate.
pureProp :: (a -> Bool) -> a -> PropM Bool
pureProp p = pure . p

-- | Runs a 'PropM' action under an existing handle (for a thread you start
-- yourself).  If the scope that owns the handle has closed, every charge the
-- action makes is refused.
withBudget :: Budget -> PropM a -> IO a
withBudget b act = unPropM act b

-- | Forks a thread that runs the action under the SAME handle, so its checks
-- charge the same tree.  A thread started with a bare @forkIO@ has no handle and
-- gets a fresh root instead (VER05n 5.1).  The body runs UNMASKED (it does not
-- inherit a masked state) and every exception it throws is dropped: nothing
-- reaches the default stderr handler with its @show@ text.  The child is not
-- killed when the scope closes; it keeps a spent handle, so every charge it makes
-- is refused.
forkWithBudget :: PropM () -> PropM ThreadId
forkWithBudget = forkWithBudgetHandler (\_ -> pure ())

-- | 'forkWithBudget' with a caller-supplied handler, which receives only the
-- exception's TYPE name (never its message).
forkWithBudgetHandler :: (T.Text -> IO ()) -> PropM () -> PropM ThreadId
forkWithBudgetHandler onError act =
  PropM
    ( \b ->
        forkIOWithUnmask
          ( \unmask -> do
              r <- try (unmask (unPropM act b))
              case r of
                Left e -> onError (exceptionTypeName e) `catch` \(_ :: SomeException) -> pure ()
                Right () -> pure ()
          )
    )

-- | A top-level check: a fresh root, default shrinker, no override.
check :: (NFData a, Canonical a) => Word64 -> Gen a -> Options -> (a -> PropM Bool) -> IO Verdict
check = checkWith Nothing Nothing

-- | 'check' for a pure predicate.
checkPure :: (NFData a, Canonical a) => Word64 -> Gen a -> Options -> (a -> Bool) -> IO Verdict
checkPure seed g o p = check seed g o (pureProp p)

-- | A check NESTED in the running property: a child of its scope.
checkM :: (NFData a, Canonical a) => Word64 -> Gen a -> Options -> (a -> PropM Bool) -> PropM Verdict
checkM seed g o p = askBudget >>= \b -> liftIO (checkWith (Just b) Nothing seed g o p)

-- | An approximate size of a value in elements (work): a text or byte string
-- counts its length plus, one level deeper, the length of each child that is
-- itself a collection or string, visiting at most 10,000 children.  Computed over
-- the lazy 'Val' view: a hostile @toVal@ counts 0.
valWork :: Val -> IO Int
valWork v = do
  r <- tryRecoverable (evaluate (sizeOf v))
  pure (either (const 0) id r)

sizeOf :: Val -> Int
sizeOf v = case v of
  VBytes b -> B.length b
  VText t -> B.length (TE.encodeUtf8 t)
  VList xs -> collection xs
  VTuple xs -> collection xs
  _ -> 0
  where
    collection xs =
      let kids = take 10000 xs
       in length kids + sum (map shallow kids)
    shallow k = case k of
      VBytes b -> B.length b
      VText t -> T.length t
      VList ys -> length (take 10000 ys)
      VTuple ys -> length (take 10000 ys)
      _ -> 0

-- | The general entry point: an optional parent scope and an optional override
-- of the generator's shrinker (so a defective shrinker can be tested).
checkWith ::
  (NFData a, Canonical a) =>
  Maybe Budget ->
  Maybe (Shrinker a) ->
  Word64 ->
  Gen a ->
  Options ->
  (a -> PropM Bool) ->
  IO Verdict
checkWith parent override seed gen opts prop = do
  validateOptions opts
  if numTests == 0
    then pure (failV failureExhausted "no cases were evaluated (num_tests is 0); a property that never ran is not a pass" 0 0)
    else do
      -- A check nested in a property pays one attempt at entry.  A closed or
      -- stale parent refuses it: exhausted without evaluating anything.
      entryRefused <- case parent of
        Just p -> not <$> charge p 1
        Nothing -> pure False
      if entryRefused
        then pure (spent 0)
        else do
          -- The WHOLE check (case loop included) runs in one scope.
          r <- withCheck cfg parent numTests maxSteps $ \scope -> do
            rng <- cfgNewRng (budgetConfig scope) seed
            runCases scope rng 1
          pure (maybe (failV failureExhausted "check nesting is too deep; nothing was evaluated" 0 0) id r)
  where
    redact = optCi opts /= CiOff
    numTests = optNumTests opts
    maxSteps = optMaxShrinkSteps opts
    cfg = optConfig opts
    nested = maybe False (const True) parent

    runCases scope rng i
      | i > numTests = finish scope
      | otherwise = do
          -- one attempt per case after the first (the first was paid at entry)
          refusedCase <- if nested && i > 1 then not <$> charge scope 1 else pure False
          if refusedCase then pure (spent i) else runCase scope rng i

    -- A top-level check that would pass but whose tree refused a charge (a
    -- starved nested check or generator) was not fully evaluated: never a pass.
    finish scope = do
      refused <- isRefused scope
      if nested || not refused
        then pure (Verdict True Nothing Nothing numTests 0 0 Nothing Nothing Nothing Nothing)
        else pure (failV failureExhausted "a nested check was starved by the shrink budget; the property was not fully evaluated" numTests numTests)

    runCase scope rng i = do
      -- the candidate is forced to normal form INSIDE the sampling handler
      s <- tryRecoverable (genSample gen (Just scope) rng >>= \v -> evaluate (force v))
      case s of
        Left e -> exhausted redact e i
        Right cand -> do
          w <- valWork (toVal cand)
          starved <- if nested then not <$> chargeCopy scope w else pure False
          if starved
            then pure (spent i)
            else do
              p <- tryRecoverable (unPropM (prop cand) scope >>= evaluate)
              case p of
                Left e -> panic seed redact e i
                Right True -> runCases scope rng (i + 1)
                Right False -> falsified scope cand i

    -- True while the candidate still falsifies.  A candidate on which the
    -- property throws is a different failure, never a smaller counterexample.
    falsifiesIn scope v = do
      r <- tryRecoverable (unPropM (prop v) scope >>= evaluate)
      pure (either (const False) not r)

    falsified scope cand i = do
      let f = falsifiesIn scope
      -- Untrusted shrinker: fall back to the original.  The shrink scope is a
      -- child of the check scope (itself a child of any enclosing shrink): every
      -- evaluation is charged to the whole chain.
      shrunk <- tryRecoverable $
        withChild cfg (Just scope) maxSteps $ \sc -> do
          let charged = chargedFalsifies sc f
          out <- case override of
            Just s -> s sc cand charged maxSteps
            Nothing -> shrinkNested sc gen cand charged maxSteps
          evaluate (force out)
      (minimal, steps) <- case shrunk of
        Right (val, used)
          | used >= 0 && used <= maxSteps -> do
              -- Re-verify: the witness we report must really fail the property.
              -- One attempt is charged to the check scope; if the chain refuses
              -- it the shrunk value is not adopted.
              ok <- charge scope 1
              still <- if ok then f val else pure False
              pure (if still then (val, used) else (cand, 0))
        _ -> pure (cand, 0)
      minimalText <- render (toVal minimal)
      if redact
        then do
          let preimage = "iter:" ++ show i ++ ";steps:" ++ show steps ++ ";shrunk:" ++ T.unpack minimalText
              digest = hex16 (fnv1a64 (TE.encodeUtf8 (T.pack preimage)))
              msg = "property falsified after " ++ show i ++ " iterations (shrunk in " ++ show steps ++ " steps, repro-fnv: " ++ digest ++ ")"
              base = failV failureCounterexample (T.pack msg) i i
          pure base {vShrinkSteps = steps, vDigest = Just (T.pack digest)}
        else do
          initialText <- render (toVal cand)
          let msg =
                "property falsified at iteration " <> T.pack (show i) <> " (seed=" <> T.pack (show seed) <> "): initial="
                  <> initialText <> ", minimal=" <> minimalText <> " after " <> T.pack (show steps) <> " shrink steps"
              base = failV failureCounterexample msg i i
          pure base {vShrinkSteps = steps, vSeed = Just seed, vCounterexample = Just initialText, vMinimal = Just minimalText}

failV :: Text -> Text -> Int -> Int -> Verdict
failV fid msg cases iter = Verdict False (Just fid) (Just msg) cases iter 0 Nothing Nothing Nothing Nothing

-- | The enclosing shrink budget was spent: exhausted, never a pass.
spent :: Int -> Verdict
spent i = failV failureExhausted ("the enclosing shrink budget was spent at iteration " <> T.pack (show i)) (max 0 (i - 1)) i

-- | A generator error: a filter that never accepts, a spent budget, or any other
-- exception from the sampler (a forced lazy one included).
exhausted :: Bool -> SomeException -> Int -> IO Verdict
exhausted redact e i = do
  text <- case fromException e of
    Just (ExhaustedError _) -> pure ("filter precondition rejected all candidates at iteration " <> T.pack (show i))
    _
      | redact -> pure ("generator error at iteration " <> T.pack (show i) <> " (" <> exceptionTypeName e <> ")")
      | otherwise -> do
          m <- safeMessage e
          pure ("generator error at iteration " <> T.pack (show i) <> ": " <> m)
  pure (failV failureExhausted text (i - 1) i)

-- | The property threw (or its 'Bool' was a lazy bottom).  Redacted messages
-- name only the exception's type; the seed appears only unredacted.
panic :: Word64 -> Bool -> SomeException -> Int -> IO Verdict
panic seed redact e i
  | redact = pure (failV failurePanic ("property evaluation raised at iteration " <> T.pack (show i) <> " (" <> exceptionTypeName e <> ")") i i)
  | otherwise = do
      m <- safeMessage e
      let base = failV failurePanic ("property evaluation raised at iteration " <> T.pack (show i) <> ": " <> m) i i
      pure base {vSeed = Just seed}

hex16 :: Word64 -> String
hex16 w = let h = showHex w "" in replicate (16 - length h) '0' ++ h
