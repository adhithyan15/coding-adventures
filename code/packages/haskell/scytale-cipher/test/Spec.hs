import qualified GeneratedClassicalCipherFixtureSpec
import qualified ScytaleCipherSpec
import Test.Hspec (hspec)

main :: IO ()
main = hspec $ do
    ScytaleCipherSpec.spec
    GeneratedClassicalCipherFixtureSpec.spec
