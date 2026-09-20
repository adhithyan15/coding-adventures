import DerTlvSpec (spec)
import qualified PortableConformanceSpec
import Test.Hspec (hspec)

main :: IO ()
main = hspec (spec >> PortableConformanceSpec.spec)
