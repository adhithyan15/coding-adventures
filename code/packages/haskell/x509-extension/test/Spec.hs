import Test.Hspec (hspec)
import qualified PortableConformanceSpec
import qualified X509ExtensionSpec

main :: IO ()
main = hspec $ do
  X509ExtensionSpec.spec
  PortableConformanceSpec.spec
