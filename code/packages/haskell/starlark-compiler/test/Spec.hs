import qualified STP01ConformanceSpec
import qualified StarlarkCompilerSpec
import Test.Hspec (hspec)

main :: IO ()
main = hspec $ do
    StarlarkCompilerSpec.spec
    STP01ConformanceSpec.spec
