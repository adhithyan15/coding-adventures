import qualified Bezier2DSpec
import qualified FlatteningConformanceSpec
import Test.Hspec

main :: IO ()
main = hspec $ do
    Bezier2DSpec.spec
    FlatteningConformanceSpec.spec
