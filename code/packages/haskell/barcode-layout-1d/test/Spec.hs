import BarcodeLayout1DSpec (spec)
import qualified BarcodeLayout1DConformanceSpec
import Test.Hspec (hspec)

main :: IO ()
main = hspec $ do
  spec
  BarcodeLayout1DConformanceSpec.spec
