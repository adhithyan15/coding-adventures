-- | The value model the engine renders and compares.
--
-- Rendering a witness must never call @show@ on a user value: a user @Show@
-- instance can loop, throw, or print forever.  Instead a user type says how to
-- view itself as a 'Val' (a closed, structural tree) through 'Canonical', and
-- the engine walks THAT, with depth, node and output caps enforced as it goes.
--
-- 'VList' and 'VTuple' hold lazy lists on purpose: an infinite or throwing
-- tail is a legal input to the renderer, which stops at its caps.
module CodingAdventures.TestBootstrap.CorpusJson.Val
  ( Val (..)
  , Canonical (..)
  ) where

import Data.ByteString (ByteString)
import Data.Text (Text)
import Data.Word (Word64, Word8)

data Val
  = VNil
  | VBool !Bool
  | VInt !Integer
  | VBytes !ByteString
  | VText !Text
  | VList [Val]
  | VTuple [Val]

class Canonical a where
  toVal :: a -> Val

instance Canonical Val where toVal = id
instance Canonical () where toVal () = VNil
instance Canonical Bool where toVal = VBool
instance Canonical Integer where toVal = VInt
instance Canonical Int where toVal = VInt . toInteger
instance Canonical Word8 where toVal = VInt . toInteger
instance Canonical Word64 where toVal = VInt . toInteger
instance Canonical ByteString where toVal = VBytes
instance Canonical Text where toVal = VText
instance Canonical a => Canonical [a] where toVal = VList . map toVal
instance Canonical a => Canonical (Maybe a) where
  toVal Nothing = VNil
  toVal (Just x) = VTuple [toVal x]
instance (Canonical a, Canonical b) => Canonical (a, b) where
  toVal (a, b) = VTuple [toVal a, toVal b]
instance (Canonical a, Canonical b, Canonical c) => Canonical (a, b, c) where
  toVal (a, b, c) = VTuple [toVal a, toVal b, toVal c]
