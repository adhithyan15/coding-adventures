import Test.Hspec
import qualified PortableConformanceSpec
import ZipSpec

main :: IO ()
main = hspec $ do
    spec
    PortableConformanceSpec.spec
