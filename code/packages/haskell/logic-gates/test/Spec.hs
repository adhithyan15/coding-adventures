import Test.Hspec
import LogicGatesSpec
import qualified HardenSpec

main :: IO ()
main = hspec (spec >> HardenSpec.spec)
