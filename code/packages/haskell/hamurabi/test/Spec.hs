import Test.Hspec
import qualified ConformanceSpec
import qualified HamurabiSpec

main :: IO ()
main = hspec $ do
  HamurabiSpec.spec
  ConformanceSpec.spec
