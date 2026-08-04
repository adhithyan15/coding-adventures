import Test.Hspec

import BuildToolSpec (buildToolSpec)
import HashingSpec (hashingSpec)
import ResolutionUtf8Spec
    ( resolutionCabalSpec
    , resolutionDartSpec
    , resolutionElixirSpec
    , resolutionGoSpec
    , resolutionGradleSpec
    , resolutionPerlSpec
    , resolutionPythonSpec
    , resolutionRubySpec
    , resolutionRustSpec
    , resolutionSwiftSpec
    , resolutionUtf8Spec
    )

main :: IO ()
main = hspec spec

spec :: Spec
spec = do
    buildToolSpec
    hashingSpec
    resolutionUtf8Spec
    resolutionCabalSpec
    resolutionGoSpec
    resolutionElixirSpec
    resolutionDartSpec
    resolutionGradleSpec
    resolutionPythonSpec
    resolutionRustSpec
    resolutionRubySpec
    resolutionPerlSpec
    resolutionSwiftSpec
