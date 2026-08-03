import Test.Hspec

import BuildToolSpec (buildToolSpec)
import HashingSpec (hashingSpec)
import ResolutionUtf8Spec (resolutionCabalSpec, resolutionPythonSpec, resolutionRubySpec, resolutionRustSpec, resolutionUtf8Spec)

main :: IO ()
main = hspec spec

spec :: Spec
spec = do
    buildToolSpec
    hashingSpec
    resolutionUtf8Spec
    resolutionCabalSpec
    resolutionPythonSpec
    resolutionRustSpec
    resolutionRubySpec
