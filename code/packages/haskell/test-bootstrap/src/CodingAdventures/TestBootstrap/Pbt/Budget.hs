-- | The budget of a shrink, as a TREE of scopes (VER05n 5, ported from
-- VER05e 4.1).
--
-- Why a tree?  A property may itself call @check@, a custom shrinker may call
-- @shrinkGen@, a filter predicate may do either.  If each of those got a fresh
-- budget the work would multiply with the nesting depth.  Here every public
-- entry point that runs user code opens a CHILD of the scope it is given, with
-- its own local cap, and a charge walks from the node up to the outermost
-- root: every node on the path must have room, then every counter is
-- incremented, all or nothing.  Whatever the nesting, the total work of one
-- top-level check is capped by the outermost root.
--
-- Two things are charged:
--
-- * ATTEMPTS: one per property evaluation;
-- * WORK: elements copied, at least 1 per charge.
--
-- /Why STM./  A charge is ONE @atomically@ transaction over the whole path.
-- There is no lock to order or leak, no reservation to roll back, and no
-- operation here ever @retry@s, so nothing can block (and nothing is an
-- interruption point that could leave a counter half updated).  A scope is
-- closed in a @bracket@ release, so it closes however the body is left,
-- asynchronous exceptions included.
--
-- /Explicit, not ambient./  There is no thread-local: every function takes the
-- parent scope as a @Maybe Budget@ ('Nothing' = a fresh root).  A thread that
-- is not handed the handle therefore gets a fresh root; see VER05n 5.1.
--
-- /Spent handles./  A closed node, or a node with a closed ancestor, refuses
-- every charge.  There is no fallback to an open ancestor and a closed handle
-- is never a way to obtain a fresh root.
module CodingAdventures.TestBootstrap.Pbt.Budget
  ( Budget
  , withRoot
  , withChild
  , withCheck
  , charge
  , chargeCopy
  , room
  , attempts
  , work
  , isClosed
  , isRefused
  , markRefused
  , isNested
  , budgetDepth
  , budgetChecks
  , budgetConfig
  , maxAttemptsOf
  , maxWorkOf
  , chargedFalsifies
  ) where

import Control.Concurrent.STM
import Control.Exception (bracket)

import CodingAdventures.TestBootstrap.Pbt.Types

data Budget = Budget
  { bParent :: !(Maybe Budget)
  , bDepth :: !Int
  , bChecks :: !Int
  , bMaxAttempts :: !Int
  , bMaxWork :: !Int
  , bAttempts :: !(TVar Int)
  , bWork :: !(TVar Int)
  , bClosed :: !(TVar Bool)
  , bRefused :: !(TVar Bool)
  -- ^ shared by the whole tree: children alias the root's cell
  , bConfig :: Config
  }

newNode :: Config -> Maybe Budget -> Int -> Int -> Int -> IO Budget
newNode cfg parent maxA maxW checks = do
  a <- newTVarIO 0
  w <- newTVarIO 0
  c <- newTVarIO False
  r <- maybe (newTVarIO False) (pure . bRefused) parent
  pure
    Budget
      { bParent = parent
      , bDepth = maybe 0 ((+ 1) . bDepth) parent
      , bChecks = checks
      , bMaxAttempts = maxA
      , bMaxWork = maxW
      , bAttempts = a
      , bWork = w
      , bClosed = c
      , bRefused = r
      , bConfig = maybe cfg bConfig parent
      }

-- | Opens an OWNING scope for the duration of the body and closes it in a
-- @bracket@ release (one non-blocking transaction), whatever way the body is left.
owned :: IO Budget -> (Budget -> IO a) -> IO a
owned mk = bracket mk (\b -> atomically (writeTVar (bClosed b) True))

-- | A fresh ROOT with the given attempt cap and the configuration's work cap.
withRoot :: Config -> Int -> (Budget -> IO a) -> IO a
withRoot cfg maxA = owned (newNode cfg Nothing (max 0 maxA) (cfgMaxWork cfg) 0)

-- | The scope of a public entry point that shrinks: a CHILD of the parent with a
-- fresh local cap (@16 * max_steps + 64@ attempts), or a fresh root when there
-- is none.  At depth 'maxBudgetDepth' the call JOINS the parent instead of
-- nesting (nothing is opened, nothing is closed), which keeps the walk short.
withChild :: Config -> Maybe Budget -> Int -> (Budget -> IO a) -> IO a
withChild cfg Nothing maxSteps body = owned (newNode cfg Nothing (attemptBudget maxSteps) (cfgMaxWork cfg) 0) body
withChild cfg (Just p) maxSteps body
  | bDepth p + 1 >= maxBudgetDepth = body p
  | otherwise = owned (newNode cfg (Just p) (attemptBudget maxSteps) maxShrinkWork (bChecks p)) body

-- | The scope of a whole check (case loop included): a root with no parent, a
-- child otherwise, with attempt cap @num_tests + 16 * max_steps + 64@.  Returns
-- 'Nothing' WITHOUT running the body (after marking the tree refused) when the
-- parent already has 'maxCheckNesting' enclosing checks or sits so deep that a
-- new scope would merely join it: such a check evaluates nothing, so the run can
-- never pass.
withCheck :: Config -> Maybe Budget -> Int -> Int -> (Budget -> IO a) -> IO (Maybe a)
withCheck cfg Nothing numTests maxSteps body =
  Just <$> owned (newNode cfg Nothing (numTests + attemptBudget maxSteps) (cfgMaxWork cfg) 1) body
withCheck cfg (Just p) numTests maxSteps body
  | bChecks p >= maxCheckNesting || bDepth p + 1 >= maxBudgetDepth = markRefused p >> pure Nothing
  | otherwise =
      Just <$> owned (newNode cfg (Just p) (numTests + attemptBudget maxSteps) maxShrinkWork (bChecks p + 1)) body

-- | One attempt over a candidate of @elements@ elements (at least 1) charged to
-- this node and every ancestor.  False, charging NOTHING, when any of them
-- cannot afford it.  Callers must charge BEFORE building the candidate.
charge :: Budget -> Int -> IO Bool
charge b elements = take' b elements 1

-- | A copy that is not itself a property evaluation: work only.
chargeCopy :: Budget -> Int -> IO Bool
chargeCopy b elements = take' b elements 0

take' :: Budget -> Int -> Int -> IO Bool
take' node elements attemptCost = atomically $ do
  let cost = max 1 elements
  ok <- pathHasRoom node cost
  if ok
    then bump node cost >> pure True
    else writeTVar (bRefused node) True >> pure False
  where
    bump n cost = do
      modifyTVar' (bAttempts n) (+ attemptCost)
      modifyTVar' (bWork n) (+ cost)
      maybe (pure ()) (`bump` cost) (bParent n)

-- | Every node on the path is open, under its attempt cap, and can afford the
-- cost.  The affordability test is @cost <= max - used@: @used <= max@ always,
-- so the subtraction cannot go negative or wrap, and @maxBound@ is refused.
pathHasRoom :: Budget -> Int -> STM Bool
pathHasRoom n cost = do
  closed <- readTVar (bClosed n)
  a <- readTVar (bAttempts n)
  w <- readTVar (bWork n)
  let here = not closed && a < bMaxAttempts n && cost <= bMaxWork n - w
  if not here
    then pure False
    else maybe (pure True) (`pathHasRoom` cost) (bParent n)

-- | True while at least one more attempt of one element could be afforded
-- everywhere on the path.
room :: Budget -> IO Bool
room b = atomically (pathHasRoom b 1)

attempts :: Budget -> IO Int
attempts = readTVarIO . bAttempts

work :: Budget -> IO Int
work = readTVarIO . bWork

-- | True once this scope has been closed (its own flag).
isClosed :: Budget -> IO Bool
isClosed = readTVarIO . bClosed

-- | True once any charge anywhere in this scope's tree has been refused.
isRefused :: Budget -> IO Bool
isRefused = readTVarIO . bRefused

markRefused :: Budget -> IO ()
markRefused b = atomically (writeTVar (bRefused b) True)

-- | True when this scope has an owning parent (it is not an outermost root).
isNested :: Budget -> Bool
isNested b = case bParent b of
  Nothing -> False
  Just _ -> True

budgetDepth :: Budget -> Int
budgetDepth = bDepth

budgetChecks :: Budget -> Int
budgetChecks = bChecks

budgetConfig :: Budget -> Config
budgetConfig = bConfig

maxAttemptsOf :: Budget -> Int
maxAttemptsOf = bMaxAttempts

maxWorkOf :: Budget -> Int
maxWorkOf = bMaxWork

-- | Wraps a falsifies function so each evaluation is charged one attempt first
-- (False once refused, WITHOUT running the property).
chargedFalsifies :: Budget -> (a -> IO Bool) -> a -> IO Bool
chargedFalsifies b f v = do
  ok <- charge b 1
  if ok then f v else pure False
