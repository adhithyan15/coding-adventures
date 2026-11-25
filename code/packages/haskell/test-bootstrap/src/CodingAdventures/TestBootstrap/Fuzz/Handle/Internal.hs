-- | UNSAFE INTERNALS of the fuzz handle (VER06o 5.3).  NOT FOR TARGETS.
--
-- This module exposes the handle's representation so the engine's own tests can
-- forge a handle (another token over the same state) and clear the sticky flag to
-- prove the counter cross-check.  A target that imports it can do the same to the
-- engine, which is the documented in-process limit (the trusted-target model):
-- inside one GHC process a hostile target can also call @setAllocationCounter@ or
-- @disableAllocationLimit@ to fake or hide an oom.  The public module
-- "CodingAdventures.TestBootstrap.Fuzz.Handle" exports 'Handle' abstractly.
module CodingAdventures.TestBootstrap.Fuzz.Handle.Internal
  ( Handle (..)
  , HState (..)
  , Exhaustion (..)
  ) where

import Data.IORef (IORef)
import qualified Data.IntMap.Strict as IM
import Data.Word (Word64)

data Exhaustion = ExNone | ExSteps | ExMem
  deriving (Eq, Show)

data HState = HState
  { hsLive :: !Bool
  , hsToken :: !Int
  , hsStepBudget :: !Int
  , hsMemBudget :: !Int
  , hsSteps :: !Int
  , hsMem :: !Int
  , hsExhausted :: !Exhaustion
  , hsEdgeCalls :: !Int
  , hsMap :: !(IM.IntMap Word64)
  }

data Handle = Handle {hRef :: !(IORef HState), hToken :: !Int}

instance Show Handle where
  show _ = "Handle<>"
