module Main (main) where

import Control.Exception (evaluate)
import Control.Monad (unless)
import qualified Data.ByteString as BS
import qualified Data.ByteString.Char8 as BS8
import Data.List (foldl')
import Data.Word (Word64)
import GHC.Stats (RTSStats (allocated_bytes), getRTSStats, getRTSStatsEnabled)
import Sha256 (Sha256Context, sha256FinalizeHex, sha256Init, sha256Update)
import System.Mem (performGC)

allocationLimit :: Word64
allocationLimit = 128 * 1024 * 1024

expectedDigest :: String
expectedDigest = "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"

main :: IO ()
main = do
    statsEnabled <- getRTSStatsEnabled
    unless statsEnabled (fail "allocation-budget: RTS statistics are not enabled")

    let payload = BS8.replicate 1000000 'a'
    _ <- evaluate (BS.length payload)

    assertAllocationBudget "one million-byte chunk" (oneChunk payload)
    assertAllocationBudget "bounded 8 KiB chunks" (boundedChunks payload)

oneChunk :: BS.ByteString -> String
oneChunk = sha256FinalizeHex . sha256Update sha256Init

boundedChunks :: BS.ByteString -> String
boundedChunks = sha256FinalizeHex . feed sha256Init
  where
    feed :: Sha256Context -> BS.ByteString -> Sha256Context
    feed context remaining
        | BS.null remaining = context
        | otherwise =
            let (chunk, rest) = BS.splitAt 8192 remaining
                next = sha256Update context chunk
             in next `seq` feed next rest

assertAllocationBudget :: String -> String -> IO ()
assertAllocationBudget label actualDigest = do
    performGC
    before <- getRTSStats
    _ <- evaluate (foldl' (\count character -> count + fromEnum character) 0 actualDigest)
    unless (actualDigest == expectedDigest) $
        fail (label ++ ": digest mismatch: " ++ actualDigest)
    performGC
    after <- getRTSStats

    let allocated = allocated_bytes after - allocated_bytes before
    putStrLn (label ++ ": allocated " ++ show allocated ++ " bytes")
    unless (allocated <= allocationLimit) $
        fail
            ( label
                ++ ": allocation budget exceeded: "
                ++ show allocated
                ++ " > "
                ++ show allocationLimit
            )
