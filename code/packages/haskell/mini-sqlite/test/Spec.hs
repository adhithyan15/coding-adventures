import Test.Hspec
import MiniSqliteSpec
import ConformanceSpec

main :: IO ()
main = hspec $ do
    spec
    conformanceSpec
