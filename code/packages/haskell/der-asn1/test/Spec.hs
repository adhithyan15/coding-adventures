module Main (main) where

import Test.Hspec (hspec)
import qualified DerAsn1Spec
import qualified PortableConformanceSpec

main :: IO ()
main = hspec $ do
  DerAsn1Spec.spec
  PortableConformanceSpec.spec
