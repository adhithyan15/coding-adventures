import Test.Hspec

import qualified CompilerSpec
import qualified CrossValidatorSpec
import qualified ParserGrammarSpec
import qualified TokenGrammarSpec

main :: IO ()
main =
    hspec $ do
        TokenGrammarSpec.spec
        ParserGrammarSpec.spec
        CrossValidatorSpec.spec
        CompilerSpec.spec
