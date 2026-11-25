-- | Finds and reads the committed shared fixtures by walking up from the working
-- directory to the repository root (never a copy): the pbt-v1 and fuzz-v1 corpora
-- and the Go fuzz engine's reference-vector table.
module Fixture
  ( readCorpus
  , corpusPath
  , readTfv
  , readFuzzCorpus
  , readFuzzTfv
  , readGoVectors
  ) where

import qualified Data.ByteString as B
import System.Directory (canonicalizePath, doesFileExist)
import System.FilePath (takeDirectory, (</>))

-- | Walks up until @<root>/<rel>@ exists.
findUnderRoot :: [FilePath] -> IO FilePath
findUnderRoot rel = canonicalizePath "." >>= go (12 :: Int)
  where
    go 0 _ = ioError (userError ("fixture not found: " ++ foldr1 (</>) rel))
    go n dir = do
      let candidate = foldl (</>) dir rel
      ok <- doesFileExist candidate
      if ok
        then pure candidate
        else
          let parent = takeDirectory dir
           in if parent == dir then ioError (userError ("fixture not found: " ++ foldr1 (</>) rel)) else go (n - 1) parent

findFixture :: String -> FilePath -> IO FilePath
findFixture dir name = findUnderRoot ["code", "specs", "fixtures", dir, name]

corpusPath :: IO FilePath
corpusPath = findFixture "pbt-v1" "cases.json"

readCorpus :: IO B.ByteString
readCorpus = corpusPath >>= B.readFile

readTfv :: IO B.ByteString
readTfv = findFixture "pbt-v1" "cases.tfv" >>= B.readFile

readFuzzCorpus :: IO B.ByteString
readFuzzCorpus = findFixture "fuzz-v1" "cases.json" >>= B.readFile

readFuzzTfv :: IO B.ByteString
readFuzzTfv = findFixture "fuzz-v1" "cases.tfv" >>= B.readFile

-- | The Go lane's committed oracle table (parsed by "Vectors", never copied).
readGoVectors :: IO B.ByteString
readGoVectors = findUnderRoot ["code", "packages", "go", "test-bootstrap", "fuzz", "reference_vectors_test.go"] >>= B.readFile
