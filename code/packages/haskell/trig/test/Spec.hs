import Test.Hspec (hspec)
import qualified ConformanceSpec
import qualified TrigSpec

main :: IO ()
main = hspec (TrigSpec.spec >> ConformanceSpec.spec)
