import Test.Hspec

import BuildToolSpec (buildToolSpec)

main :: IO ()
main = hspec spec

spec :: Spec
spec = buildToolSpec
