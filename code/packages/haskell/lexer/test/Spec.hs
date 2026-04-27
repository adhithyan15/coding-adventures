import Test.Hspec

import qualified LexerSpec
import qualified TokenizerDFASpec

main :: IO ()
main = hspec spec

spec :: Spec
spec = do
    LexerSpec.spec
    TokenizerDFASpec.spec
