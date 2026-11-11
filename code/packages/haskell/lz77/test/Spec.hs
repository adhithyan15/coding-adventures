import qualified HostileHeadersSpec
import qualified LZ77Spec
import Test.Hspec (hspec)

main :: IO ()
main = hspec (LZ77Spec.spec >> HostileHeadersSpec.spec)
