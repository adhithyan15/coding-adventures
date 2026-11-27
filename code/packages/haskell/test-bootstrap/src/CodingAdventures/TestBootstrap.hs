-- | The Haskell property-based testing engine (VER05n).
--
-- One import for everything a test author needs:
--
-- > import CodingAdventures.TestBootstrap
-- >
-- > main = do
-- >   v <- checkPure 42 (int 0 100000) defaultOptions {optCi = CiOff} (<= 100)
-- >   print (vFailureId v, vMinimal v)   -- (Just "property-counterexample", Just "101")
--
-- The lower-level modules stay available for the conformance runner and the
-- negative controls.
module CodingAdventures.TestBootstrap
  ( module CodingAdventures.TestBootstrap.Error
  , module CodingAdventures.TestBootstrap.Prng
  , module CodingAdventures.TestBootstrap.Pbt.Types
  , module CodingAdventures.TestBootstrap.Pbt.Budget
  , module CodingAdventures.TestBootstrap.Pbt.Shrink
  , module CodingAdventures.TestBootstrap.Pbt.Gen
  , module CodingAdventures.TestBootstrap.Pbt.Runner
  , module CodingAdventures.TestBootstrap.CorpusJson.Val
  , module CodingAdventures.TestBootstrap.CorpusJson.Canon
  ) where

import CodingAdventures.TestBootstrap.CorpusJson.Canon (render, renderText)
import CodingAdventures.TestBootstrap.CorpusJson.Val
import CodingAdventures.TestBootstrap.Error
import CodingAdventures.TestBootstrap.Pbt.Budget
import CodingAdventures.TestBootstrap.Pbt.Gen
import CodingAdventures.TestBootstrap.Pbt.Runner
import CodingAdventures.TestBootstrap.Pbt.Shrink
import CodingAdventures.TestBootstrap.Pbt.Types
import CodingAdventures.TestBootstrap.Prng
