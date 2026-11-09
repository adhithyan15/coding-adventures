import qualified HostileHeadersSpec
import qualified LZWSpec
import Test.Hspec (hspec)

main :: IO ()
main = hspec (LZWSpec.spec >> HostileHeadersSpec.spec)
