-- | The per-execution coverage and budget handle (VER06o 4.2 and 5.3).
--
-- A target receives a FRESH handle for every execution and reports coverage with
-- 'edge', burns its step budget with 'tick' and its memory budget with 'reserve'.
-- All state lives in ONE 'IORef' holding an immutable strict record, changed only
-- with 'atomicModifyIORef'', so any thread holding the handle acts on the same
-- state atomically and there is no lock a killed thread could leave held.
--
-- The state:
--
-- > live / retired         the identity token hash
-- > step and memory budgets and counters
-- > exhausted: none | steps | memory     (STICKY: the first breach wins)
-- > number of 'edge' calls
-- > the 65,536-bit edge map as at most 1,024 unsigned 64-bit words
--
-- Every call first checks that the handle is live AND that its token matches the
-- stored one, so a handle rebuilt with another token (or a retired one) is inert.
--
-- The type is exported ABSTRACTLY; the representation lives in
-- "CodingAdventures.TestBootstrap.Fuzz.Handle.Internal" (unsafe, not for targets).
--
-- HONEST LIMIT.  Inside one GHC process a hostile target can reach the 'IORef'
-- (it was handed the handle) and write any state.  The engine cross-checks the
-- counters against the budgets after the run ('exhausted'), which catches casual
-- clearing, not a determined in-process attacker.  Targets are trusted not to
-- attack the harness, as in every other lane.  The same trust covers the worker's
-- allocation counter: a target can call @setAllocationCounter@ or
-- @disableAllocationLimit@ on its own thread to fake or hide an oom.
module CodingAdventures.TestBootstrap.Fuzz.Handle
  ( Handle
  , Exhaustion (..)
  , Which (..)
  , BudgetSentinel (..)
  , newHandle
  , edgeId
  , edge
  , tick
  , reserve
  , retire
  , isRetired
  , exhausted
  , edgeCalls
  , stepsUsed
  , memUsed
  , edges
  ) where

import Control.Exception (Exception, throwIO)
import Control.Monad (forM_, when)
import Data.Bits (shiftL, shiftR, testBit, (.&.), (.|.))
import qualified Data.ByteString as B
import Data.IORef
import qualified Data.IntMap.Strict as IM
import Data.Unique (hashUnique, newUnique)
import Data.Word (Word64)

import CodingAdventures.TestBootstrap.Error (argError)
import CodingAdventures.TestBootstrap.Fuzz.Handle.Internal
import CodingAdventures.TestBootstrap.Fuzz.Limits
import CodingAdventures.TestBootstrap.Prng (fnv1a64)

-- | Which budget a sentinel reports.  'WhichRetired' is the retired handle's.
data Which = WhichSteps | WhichMem | WhichRetired
  deriving (Eq, Show)

-- | The sentinel a breaching or retired call throws: the handle's token hash and
-- the reason.  An ordinary synchronous exception (a target CAN catch it; the
-- guarantee is stickiness plus the post-run verdict, VER06o 5.2).
data BudgetSentinel = BudgetSentinel !Int !Which
  deriving (Eq, Show)

instance Exception BudgetSentinel

-- | A fresh, live handle with the given budgets (engine use).
newHandle :: Int -> Int -> IO Handle
newHandle stepBudget memBudget = do
  when (stepBudget < 0 || stepBudget > maxStepBudget) (argError "step_budget out of range")
  when (memBudget < 0 || memBudget > maxMemBudget) (argError "mem_budget out of range")
  tok <- hashUnique <$> newUnique
  ref <-
    newIORef
      HState
        { hsLive = True
        , hsToken = tok
        , hsStepBudget = stepBudget
        , hsMemBudget = memBudget
        , hsSteps = 0
        , hsMem = 0
        , hsExhausted = ExNone
        , hsEdgeCalls = 0
        , hsMap = IM.empty
        }
  pure (Handle ref tok)

-- | A stable edge id derived from a name (FNV-1a): deterministic across runs and
-- machines.
edgeId :: B.ByteString -> Integer
edgeId = toInteger . fnv1a64

valid :: Handle -> HState -> Bool
valid h s = hsLive s && hsToken s == hToken h

-- | Records that edge @id@ ran (reduced modulo 65,536).  A no-op on a retired or
-- forged handle: a write that raced the retirement lands in state nobody reads.
edge :: Handle -> Integer -> IO ()
edge h ident = atomicModifyIORef' (hRef h) $ \s ->
  if valid h s
    then
      let e = fromInteger (ident `mod` toInteger coverageMapSize) :: Int
          word = e `shiftR` 6
          bit = 1 `shiftL` (e .&. 63) :: Word64
       in (s {hsEdgeCalls = hsEdgeCalls s + 1, hsMap = IM.insertWith (.|.) word bit (hsMap s)}, ())
    else (s, ())

-- | Shared by 'tick' and 'reserve': sticky first, then the charge.  @charge@
-- gives the new state and whether the budget was passed.
spend :: Handle -> Which -> (HState -> (HState, Bool)) -> IO ()
spend h which charge = do
  r <- atomicModifyIORef' (hRef h) $ \s ->
    if not (valid h s)
      then (s, Just WhichRetired)
      else case hsExhausted s of
        ExSteps -> (s, Just WhichSteps)
        ExMem -> (s, Just WhichMem)
        ExNone ->
          let (s', over) = charge s
           in if over then (s' {hsExhausted = if which == WhichSteps then ExSteps else ExMem}, Just which) else (s', Nothing)
  forM_ r (throwIO . BudgetSentinel (hToken h))

-- | Burns @n@ steps of the step budget.  Throws the budget sentinel when the
-- budget is passed, and EVERY time afterwards (sticky, whichever budget breached);
-- throws the retired sentinel on a retired or forged handle.  A negative @n@
-- charges nothing; a huge one is clamped to @budget + 1@ so the counter cannot
-- wrap.
tick :: Handle -> Int -> IO ()
tick h n = spend h WhichSteps $ \s ->
  let b = hsStepBudget s
      used = hsSteps s + max 0 (min n (b + 1))
   in (s {hsSteps = used}, used > b)

-- | Charges @n@ bytes against the memory budget (a cooperative stand-in for an
-- allocation the target can predict).  Sticky like 'tick'.
reserve :: Handle -> Int -> IO ()
reserve h n = spend h WhichMem $ \s ->
  let b = hsMemBudget s
      used = hsMem s + max 0 (min n (b + 1))
   in (s {hsMem = used}, used > b)

-- | Atomically retires the handle; returns whether it was live.
retire :: Handle -> IO Bool
retire h = atomicModifyIORef' (hRef h) (\s -> (s {hsLive = False}, hsLive s))

isRetired :: Handle -> IO Bool
isRetired h = not . hsLive <$> readIORef (hRef h)

-- | The sticky flag, cross-checked with the counters, so a target that clears the
-- flag after a breach is still judged by @steps > step_budget@ and @mem >
-- mem_budget@.
exhausted :: Handle -> IO Exhaustion
exhausted h = do
  s <- readIORef (hRef h)
  pure $ case hsExhausted s of
    ExNone
      | hsSteps s > hsStepBudget s -> ExSteps
      | hsMem s > hsMemBudget s -> ExMem
      | otherwise -> ExNone
    e -> e

edgeCalls, stepsUsed, memUsed :: Handle -> IO Int
edgeCalls h = hsEdgeCalls <$> readIORef (hRef h)
stepsUsed h = hsSteps <$> readIORef (hRef h)
memUsed h = hsMem <$> readIORef (hRef h)

-- | The distinct edge ids set so far, ascending.
edges :: Handle -> IO [Integer]
edges h = do
  s <- readIORef (hRef h)
  pure [toInteger (w * 64 + b) | (w, word) <- IM.toAscList (hsMap s), b <- [0 .. 63], testBit word b]
