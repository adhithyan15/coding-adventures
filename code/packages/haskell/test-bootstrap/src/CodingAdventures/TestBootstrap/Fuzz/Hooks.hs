-- | The sabotage seams of the fuzz corpus runner (VER06o 8).
--
-- Every engine entry point the runner calls goes through this record, so a
-- negative control can install ONE wrong function and the run must FAIL.  It is a
-- value passed down (never a global): production code uses 'defaultHooks'.
module CodingAdventures.TestBootstrap.Fuzz.Hooks
  ( Hooks (..)
  , defaultHooks
  ) where

import qualified Data.ByteString as B
import Data.Text (Text)

import CodingAdventures.TestBootstrap.Fuzz.Bitmap (corpusAdmit)
import CodingAdventures.TestBootstrap.Fuzz.FailureIds (isRegistered)
import CodingAdventures.TestBootstrap.Fuzz.Mutate
import CodingAdventures.TestBootstrap.Fuzz.Signature (duplicateCrash, formatTapFailure)
import qualified CodingAdventures.TestBootstrap.Tfv as Tfv

data Hooks = Hooks
  { hkBitFlip :: B.ByteString -> Integer -> Integer -> B.ByteString
  , hkByteFlip :: B.ByteString -> Integer -> Integer -> B.ByteString
  , hkByteArithmetic :: B.ByteString -> Integer -> Integer -> Integer -> Endian -> B.ByteString
  , hkBoundaryInsert :: B.ByteString -> Integer -> Integer -> Integer -> Endian -> B.ByteString
  , hkBlockDelete :: B.ByteString -> Integer -> Integer -> B.ByteString
  , hkBlockInsert :: B.ByteString -> Integer -> B.ByteString -> B.ByteString
  , hkBlockDuplicate :: B.ByteString -> Integer -> Integer -> Integer -> B.ByteString
  , hkSplice :: B.ByteString -> B.ByteString -> Integer -> Integer -> B.ByteString
  , hkMutateSequence :: B.ByteString -> Integer -> Integer -> Int -> Int -> IO SeqResult
  , hkCorpusAdmit :: [Integer] -> [Integer] -> (Bool, Int)
  , hkDuplicateCrash :: Text -> [Text] -> Text -> [Text] -> Bool
  , hkFailureId :: Text -> Bool
  , hkFormatTap :: Integer -> Text -> Text -> Text -> IO Text
  , hkTfvValid :: B.ByteString -> Bool
  }

defaultHooks :: Hooks
defaultHooks =
  Hooks
    { hkBitFlip = bitFlip
    , hkByteFlip = byteFlip
    , hkByteArithmetic = byteArithmetic
    , hkBoundaryInsert = boundaryInsert
    , hkBlockDelete = blockDelete
    , hkBlockInsert = blockInsert
    , hkBlockDuplicate = blockDuplicate
    , hkSplice = splice
    , hkMutateSequence = mutateSequenceCounted
    , hkCorpusAdmit = corpusAdmit
    , hkDuplicateCrash = duplicateCrash
    , hkFailureId = isRegistered
    , hkFormatTap = formatTapFailure
    , hkTfvValid = Tfv.valid
    }
