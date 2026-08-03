import Test.Hspec

import BuildToolSpec (buildToolSpec)
import ResolutionUtf8Spec (resolutionUtf8Spec)

main :: IO ()
main = hspec spec

spec :: Spec
spec = do
    buildToolSpec
    resolutionUtf8Spec
