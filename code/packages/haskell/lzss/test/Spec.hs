import qualified HostileHeadersSpec
import qualified LZSSSpec
import Test.Hspec (hspec)

main :: IO ()
main = hspec (LZSSSpec.spec >> HostileHeadersSpec.spec)
