-- | Finds and reads the committed pbt-v1 fixture by walking up from the working
-- directory to the repository root (never a copy).
module Fixture
  ( readCorpus
  , corpusPath
  , readTfv
  ) where

import qualified Data.ByteString as B
import System.Directory (canonicalizePath, doesFileExist)
import System.FilePath (takeDirectory, (</>))

-- | Walks up until @code/specs/fixtures/pbt-v1/<name>@ exists.
findFixture :: FilePath -> IO FilePath
findFixture name = canonicalizePath "." >>= go (12 :: Int)
  where
    go 0 _ = ioError (userError ("pbt-v1 fixture not found: " ++ name))
    go n dir = do
      let candidate = dir </> "code" </> "specs" </> "fixtures" </> "pbt-v1" </> name
      ok <- doesFileExist candidate
      if ok
        then pure candidate
        else
          let parent = takeDirectory dir
           in if parent == dir then ioError (userError ("pbt-v1 fixture not found: " ++ name)) else go (n - 1) parent

corpusPath :: IO FilePath
corpusPath = findFixture "cases.json"

readCorpus :: IO B.ByteString
readCorpus = corpusPath >>= B.readFile

readTfv :: IO B.ByteString
readTfv = findFixture "cases.tfv" >>= B.readFile
