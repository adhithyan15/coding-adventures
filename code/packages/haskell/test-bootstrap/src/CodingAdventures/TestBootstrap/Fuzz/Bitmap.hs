-- | The 65,536-edge execution map, admission and the power schedule
-- (VER06 4.1, VER06o 4.1).
--
-- The map is a VALUE: an 'IntSet' of edge ids reduced with the FLOORED
-- @mod 65536@ (so @-1@ is edge 65535; collisions are the known price of a
-- fixed-size map, AFL pays the same one).
module CodingAdventures.TestBootstrap.Fuzz.Bitmap
  ( Bitmap
  , emptyBitmap
  , reduceEdge
  , record
  , hasEdge
  , admit
  , coveredEdges
  , count
  , rawBitmap
  , corpusAdmit
  , calculateEnergy
  ) where

import Data.Bits (shiftL, (.|.))
import qualified Data.ByteString as B
import qualified Data.IntMap.Strict as IM
import qualified Data.IntSet as IS
import qualified Data.Set as Set
import Data.Word (Word8)

import CodingAdventures.TestBootstrap.Fuzz.Limits (coverageMapSize)

newtype Bitmap = Bitmap IS.IntSet deriving (Eq)

instance Show Bitmap where
  show (Bitmap s) = "Bitmap<edges=" ++ show (IS.size s) ++ ">"

emptyBitmap :: Bitmap
emptyBitmap = Bitmap IS.empty

-- | Floored reduction: Haskell's 'mod' floors, so a negative id wraps upward.
reduceEdge :: Integer -> Int
reduceEdge e = fromInteger (e `mod` toInteger coverageMapSize)

record :: Bitmap -> Integer -> Bitmap
record (Bitmap s) e = Bitmap (IS.insert (reduceEdge e) s)

hasEdge :: Bitmap -> Integer -> Bool
hasEdge (Bitmap s) e = IS.member (reduceEdge e) s

-- | ORs a trace into the (cumulative) map: @(has_new, count_new, map)@, over the
-- REDUCED ids.
admit :: Bitmap -> [Integer] -> (Bool, Int, Bitmap)
admit (Bitmap s) trace =
  let novel = IS.fromList (map reduceEdge trace) `IS.difference` s
      n = IS.size novel
   in (n > 0, n, Bitmap (s `IS.union` novel))

coveredEdges :: Bitmap -> [Int]
coveredEdges (Bitmap s) = IS.toAscList s

count :: Bitmap -> Int
count (Bitmap s) = IS.size s

-- | The 8,192-byte map: bit @e mod 8@ of byte @e div 8@.
rawBitmap :: Bitmap -> B.ByteString
rawBitmap (Bitmap s) =
  let bytes = IM.fromListWith (.|.) [(e `div` 8, 1 `shiftL` (e `mod` 8) :: Word8) | e <- IS.toList s]
   in B.pack [IM.findWithDefault 0 i bytes | i <- [0 .. coverageMapSize `div` 8 - 1]]

-- | Pure novelty check over the UNREDUCED ids, counting UNIQUE novel edges:
--
-- > corpusAdmit [1,2,3] [2,4]   ==  (True, 1)
-- > corpusAdmit [1,2,3] [1,2]   ==  (False, 0)
corpusAdmit :: [Integer] -> [Integer] -> (Bool, Int)
corpusAdmit existing trace =
  let novel = Set.fromList trace `Set.difference` Set.fromList existing
   in (not (Set.null novel), Set.size novel)

-- | Mutation slots for a seed: @max 1 (round_half_up (base * (1 + new/total) / sqrt len))@.
-- The edge bonus is 1.0 while nothing is covered.  Rounding is half AWAY FROM
-- ZERO (@floor (x + 0.5)@); banker's rounding would give 2 for 2.5.
--
-- Worked example: base 10, len 4, 3 new of 6 total: @10 * 1.5 / 2 = 7.5@, so 8.
calculateEnergy :: Int -> Int -> Int -> Int -> Int
calculateEnergy baseEnergy seedLen newEdges totalEdges =
  let base = fromIntegral (max 1 baseEnergy) :: Double
      bonus
        | totalEdges > 0 = 1.0 + fromIntegral newEdges / fromIntegral totalEdges
        | otherwise = 1.0
      slots = floor (base * bonus / sqrt (fromIntegral (max 1 seedLen)) + 0.5) :: Int
   in max 1 slots
