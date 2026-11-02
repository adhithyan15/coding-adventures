import Test.Hspec
import qualified ConformanceSpec
import RngSpec

main :: IO ()
main = hspec (spec >> ConformanceSpec.spec)
