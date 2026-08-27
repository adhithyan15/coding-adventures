import qualified GeneratedClassicalCipherFixtureSpec
import qualified VigenereCipherSpec
import Test.Hspec

main :: IO ()
main = hspec $ do
    VigenereCipherSpec.spec
    GeneratedClassicalCipherFixtureSpec.spec
