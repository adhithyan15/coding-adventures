import Test.Hspec
import ToolchainDetectionSpec (toolchainDetectionSpec)

import BuildToolSpec (buildToolSpec)
import GraphDiffSpec (graphDiffSpec)
import HashingSpec (hashingSpec)
import ResolutionUtf8Spec
    ( resolutionCabalSpec
    , resolutionDartSpec
    , resolutionDotnetSpec
    , resolutionElixirSpec
    , resolutionGoSpec
    , resolutionGradleSpec
    , resolutionPerlSpec
    , resolutionPythonSpec
    , resolutionRubySpec
    , resolutionRustSpec
    , resolutionSwiftSpec
    , resolutionTypescriptSpec
    , resolutionUtf8Spec
    )

main :: IO ()
main = hspec spec

spec :: Spec
spec = do
    buildToolSpec
    graphDiffSpec
    hashingSpec
    resolutionUtf8Spec
    resolutionCabalSpec
    resolutionGoSpec
    resolutionElixirSpec
    resolutionDartSpec
    resolutionGradleSpec
    resolutionDotnetSpec
    resolutionPythonSpec
    resolutionRustSpec
    resolutionRubySpec
    resolutionPerlSpec
    resolutionSwiftSpec
    resolutionTypescriptSpec
    toolchainDetectionSpec
