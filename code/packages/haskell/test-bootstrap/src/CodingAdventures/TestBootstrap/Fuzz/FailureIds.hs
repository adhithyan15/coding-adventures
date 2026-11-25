-- | The closed fuzz failure vocabulary (VER06 6.2).  Nothing else is ever
-- printed in a public failure line.
module CodingAdventures.TestBootstrap.Fuzz.FailureIds
  ( failureCrash
  , failureTimeout
  , failureOom
  , failureLeak
  , failurePanic
  , allFailureIds
  , isRegistered
  ) where

import Data.Text (Text)

failureCrash, failureTimeout, failureOom, failureLeak, failurePanic :: Text
failureCrash = "fuzz-crash"
failureTimeout = "fuzz-timeout"
failureOom = "fuzz-oom"
failureLeak = "fuzz-leak"
failurePanic = "fuzz-panic"

allFailureIds :: [Text]
allFailureIds = [failureCrash, failureTimeout, failureOom, failureLeak, failurePanic]

-- | @isRegistered "fuzz-crash"@ is true; @isRegistered "fuzz-arbitrary-error"@
-- is false.  Case and whitespace matter: @"FUZZ-CRASH"@ and @"fuzz-crash\\n"@ are not ids.
isRegistered :: Text -> Bool
isRegistered = (`elem` allFailureIds)
