import AtbashCipherSpec (spec)
import qualified GeneratedClassicalCipherFixtureSpec
import Test.Hspec (hspec)

main :: IO ()
main = hspec $ do
    spec
    GeneratedClassicalCipherFixtureSpec.spec
