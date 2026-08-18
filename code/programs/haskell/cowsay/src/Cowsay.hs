-- | cowsay — routed through paint-vm-ascii (Haskell port).
--
-- Fourth language in the cowsay-through-paint-vm-ascii rollout (after
-- csharp, fsharp, perl). Everything up through composing the bubble+cow
-- text block is ordinary string formatting, ported unchanged from the
-- reference implementation at @code\/programs\/go\/cowsay\/main.go@. The
-- one thing that's different from that reference: instead of printing the
-- composed text directly, 'buildScene' converts it into a 'PaintScene' of
-- @glyph_run@ instructions (one glyph placement per non-space character,
-- positioned on an 8x16 character grid), and
-- 'CodingAdventures.PaintVmAscii.render' turns that scene back into the
-- terminal string we print. This is also the PR that brought
-- @haskell\/paint-instructions@ and @haskell\/paint-vm-ascii@ up to the
-- full P2D02 contract — see those packages' own CHANGELOGs.
module Cowsay
  ( CowsayInvocation (..)
  , scaleXConst
  , scaleYConst
  , wrapText
  , formatBubble
  , normalizeTwoChars
  , resolveEyesAndTongue
  , loadCow
  , findRepoRoot
  , composeContent
  , buildScene
  , render
  , isListRequested
  , listCowFiles
  , resolveMessageFromArguments
  , buildInvocation
  , modeFlagIds
  ) where

import Data.List (intercalate, isPrefixOf, sort, tails)
import Data.Maybe (fromMaybe)
import qualified Data.Map.Strict as Map
import Data.Map.Strict (Map)
import System.Directory (doesFileExist, listDirectory, makeAbsolute)
import System.IO (readFile')
import System.FilePath
  ( dropExtension
  , isAbsolute
  , makeRelative
  , splitDirectories
  , takeDirectory
  , takeExtension
  , takeFileName
  , (</>)
  )

import CodingAdventures.PaintInstructions
  ( PaintGlyphPlacement (..)
  , PaintScene (..)
  , makeGlyphRun
  )
import CodingAdventures.PaintVmAscii
  ( AsciiOptions (..)
  , PaintVmAsciiError
  )
import qualified CodingAdventures.PaintVmAscii as PaintVmAscii
import JsonValue (JsonValue (..))

-- | paint-vm-ascii's documented default scale factors
-- (@P2D02-paint-vm-ascii.md@).
scaleXConst :: Int
scaleXConst = 8

scaleYConst :: Int
scaleYConst = 16

-- | The resolved set of inputs needed to render one cowsay invocation,
-- after CLI flags and mode shortcuts have been reconciled into concrete
-- values.
data CowsayInvocation = CowsayInvocation
  { ciMessage :: String
  , ciEyes :: String
  , ciTongue :: String
  , ciActiveModes :: [String]
  , ciNoWrap :: Bool
  , ciWidth :: Int
  , ciThink :: Bool
  , ciCowFile :: String
  } deriving (Eq, Show)

-- ---------------------------------------------------------------------------
-- Rendering core (ported from code/programs/go/cowsay/main.go)
-- ---------------------------------------------------------------------------

-- | Splits text into lines no longer than @width@, breaking on word
-- boundaries. A single word longer than the width is kept whole (never
-- split mid-word).
wrapText :: String -> Int -> [String]
wrapText text width
  | length text <= width = [text]
  | otherwise = case words text of
      [] -> [""]
      (w : ws) -> go ws w
  where
    go [] current = [current]
    go (w : ws) current
      | length current + length w + 1 <= width = go ws (current ++ " " ++ w)
      | otherwise = current : go ws w

-- | Draws the speech/thought bubble around the given lines. A single line
-- gets @\"< ... >\"@ (or @\"( ... )\"@ for a thought bubble); multiple
-- lines get @\"\/ ... \\\\\"@, @\"| ... |\"@, @\"\\\\ ... \/\"@ (or
-- @\"( ... )\"@ on every line for a thought bubble).
formatBubble :: [String] -> Bool -> String
formatBubble [] _ = ""
formatBubble ls isThink =
  intercalate "\n" (borderTop : body ++ [borderBottom])
  where
    maxLen = maximum (map length ls)
    borderTop = " " ++ replicate (maxLen + 2) '_'
    borderBottom = " " ++ replicate (maxLen + 2) '-'
    padded s = s ++ replicate (maxLen - length s) ' '
    body = case ls of
      [only] ->
        let (s, e) = if isThink then ("(", ")") else ("<", ">")
         in [s ++ " " ++ padded only ++ " " ++ e]
      many ->
        let n = length many
            border i
              | isThink = ("(", ")")
              | i == 0 = ("/", "\\")
              | i == n - 1 = ("\\", "/")
              | otherwise = ("|", "|")
         in [ s ++ " " ++ padded line ++ " " ++ e
            | (i, line) <- zip [0 ..] many
            , let (s, e) = border i
            ]

-- | Pads or truncates a mode string (eyes\/tongue) to exactly two
-- characters, matching cowsay's convention that eyes\/tongue are always a
-- 2-char glyph.
normalizeTwoChars :: String -> String
normalizeTwoChars s
  | length s < 2 = take 2 (s ++ "  ")
  | length s > 2 = take 2 s
  | otherwise = s

modeOverrides :: [(String, (String, Maybe String))]
modeOverrides =
  [ ("borg", ("==", Nothing))
  , ("dead", ("XX", Just "U "))
  , ("greedy", ("$$", Nothing))
  , ("paranoid", ("@@", Nothing))
  , ("stoned", ("xx", Just "U "))
  , ("tired", ("--", Nothing))
  , ("wired", ("OO", Nothing))
  , ("youthful", ("..", Nothing))
  ]

-- | Applies mode shortcuts (--borg, --dead, etc.) on top of the base
-- eyes\/tongue flag values, then normalizes both to two characters. Modes
-- are mutually exclusive per cowsay.json, but this accepts any set for
-- robustness.
resolveEyesAndTongue :: String -> String -> [String] -> (String, String)
resolveEyesAndTongue baseEyes baseTongue modes =
  (normalizeTwoChars eyes, normalizeTwoChars tongue)
  where
    (eyes, tongue) = foldl apply (baseEyes, baseTongue) modes
    apply (e, t) m = case lookup m modeOverrides of
      Nothing -> (e, t)
      Just (newEyes, Nothing) -> (newEyes, t)
      Just (newEyes, Just newTongue) -> (newEyes, newTongue)

-- | Loads a .cow template's body from @cowsDir@, falling back to
-- default.cow when the requested file doesn't exist. The template is a
-- Perl heredoc (@$the_cow = \<\<EOC; ... EOC@); only the body between the
-- heredoc markers is returned.
--
-- @cowName@ comes from the user-supplied -f\/--file flag, so it is treated
-- as untrusted: only a bare filename (no directory separators, no
-- rooted\/absolute path) is accepted, and the resolved path is verified to
-- stay inside @cowsDir@ before it's read — otherwise this falls back to
-- default.cow instead of reading an arbitrary file the caller pointed at
-- via @\"..\"@, a rooted override, or similar (mirrors the fix applied to
-- the csharp\/fsharp\/perl ports' load_cow after \/security-review).
loadCow :: String -> FilePath -> IO String
loadCow cowName cowsDir = do
  cowsRoot <- makeAbsolute cowsDir
  let safeName = takeFileName cowName
      rooted = isAbsolute cowName
  candidateMaybe <-
    if not (null safeName) && not rooted
      then Just <$> makeAbsolute (cowsRoot </> (safeName ++ ".cow"))
      else pure Nothing
  isWithin <- case candidateMaybe of
    Nothing -> pure False
    Just candidate -> do
      let relativePath = makeRelative cowsRoot candidate
      pure (".." `notElem` splitDirectories relativePath)
  existsAtCandidate <- maybe (pure False) doesFileExist candidateMaybe
  let cowPath = case candidateMaybe of
        Just candidate | isWithin && existsAtCandidate -> candidate
        _ -> cowsRoot </> "default.cow"
  contents <- readFile' cowPath
  pure (extractHeredocBody contents)

extractHeredocBody :: String -> String
extractHeredocBody contents =
  fromMaybe contents $ do
    afterStart <- afterSubstring "<<EOC;\n" contents
    beforeSubstring "EOC" afterStart

findSubstringIndex :: String -> String -> Maybe Int
findSubstringIndex needle haystack =
  go 0 (tails haystack)
  where
    go _ [] = Nothing
    go i (t : ts)
      | needle `isPrefixOf` t = Just i
      | otherwise = go (i + 1) ts

afterSubstring :: String -> String -> Maybe String
afterSubstring needle haystack = do
  idx <- findSubstringIndex needle haystack
  pure (drop (idx + length needle) haystack)

beforeSubstring :: String -> String -> Maybe String
beforeSubstring needle haystack = do
  idx <- findSubstringIndex needle haystack
  pure (take idx haystack)

-- | Walks up from @startDir@ looking for CLAUDE.md, the repo-root sentinel
-- file. CLAUDE.md (not code\/specs\/cowsay.json itself) is used
-- deliberately — it's a more robust marker than reaching for the very
-- file being located, and this exact fix was called out as a lesson from
-- a prior, reverted cowsay Lua port's CI pathing problems (PR #1535).
findRepoRoot :: FilePath -> IO FilePath
findRepoRoot startDir = go startDir (24 :: Int)
  where
    go dir remaining
      | remaining <= 0 = pure startDir
      | otherwise = do
          exists <- doesFileExist (dir </> "CLAUDE.md")
          if exists
            then pure dir
            else do
              let parent = takeDirectory dir
              if parent == dir
                then pure startDir
                else go parent (remaining - 1)

splitOnChar :: Char -> String -> [String]
splitOnChar c s = case break (== c) s of
  (chunk, []) -> [chunk]
  (chunk, _ : rest) -> chunk : splitOnChar c rest

replaceAll :: String -> String -> String -> String
replaceAll needle replacement haystack
  | null needle = haystack
  | otherwise = go haystack
  where
    go [] = []
    go s@(x : xs)
      | needle `isPrefixOf` s = replacement ++ go (drop (length needle) s)
      | otherwise = x : go xs

-- | Composes the full bubble+cow text block for one invocation —
-- everything up to (but not including) the paint-vm-ascii render step.
composeContent :: CowsayInvocation -> FilePath -> IO String
composeContent inv cowsDir = do
  cowTemplate <- loadCow (ciCowFile inv) cowsDir
  let (eyes, tongue) = resolveEyesAndTongue (ciEyes inv) (ciTongue inv) (ciActiveModes inv)
      rawLines = splitOnChar '\n' (ciMessage inv)
      ls = concatMap wrapOrKeep rawLines
      wrapOrKeep l
        | null l = [""]
        | ciNoWrap inv = [l]
        | otherwise = wrapText l (ciWidth inv)
      thoughts = if ciThink inv then "o" else "\\"
      bubble = formatBubble ls (ciThink inv)
      cow =
        replaceAll "\\\\" "\\"
          (replaceAll "$thoughts" thoughts
            (replaceAll "$tongue" tongue
              (replaceAll "$eyes" eyes cowTemplate)))
  pure (bubble ++ "\n" ++ cow)

-- | Converts a composed text block into a 'PaintScene': one @glyph_run@
-- instruction per line, one glyph placement per non-space character. See
-- @code\/specs\/cowsay-paintvm-pipeline.md@ §3 for the full contract,
-- including why glyph_id is a literal Unicode code point here (an
-- ASCII-backend-only relaxation of the general PaintGlyphRun contract).
buildScene :: String -> PaintScene
buildScene text =
  PaintScene
    { psWidth = fromIntegral (max 1 maxWidth * scaleXConst)
    , psHeight = fromIntegral (max 1 (length ls) * scaleYConst)
    , psInstructions = concatMap lineInstruction (zip [0 ..] ls)
    , psBg = "transparent"
    , psMeta = Map.empty
    }
  where
    normalized = replaceAll "\r\n" "\n" text
    ls = splitOnChar '\n' normalized
    maxWidth = maximum (0 : map length ls)
    lineInstruction (row, line)
      | null glyphs = []
      | otherwise = [makeGlyphRun glyphs "terminal-mono" (fromIntegral scaleYConst) "#000000"]
      where
        glyphs =
          [ PaintGlyphPlacement (fromEnum ch) (fromIntegral (col * scaleXConst)) (fromIntegral (row * scaleYConst))
          | (col, ch) <- zip [0 :: Int ..] line
          , ch /= ' '
          ]

-- | End-to-end: compose the bubble+cow text, build a 'PaintScene' from it,
-- and render that scene through paint-vm-ascii.
render :: CowsayInvocation -> FilePath -> IO (Either PaintVmAsciiError String)
render inv cowsDir = do
  content <- composeContent inv cowsDir
  let scene = buildScene content
  pure (PaintVmAscii.render scene (AsciiOptions scaleXConst scaleYConst))

-- ---------------------------------------------------------------------------
-- CLI glue — the bridge between CliBuilder's flags/arguments maps and the
-- typed invocation this module renders. Kept in this module (rather than
-- app/Main.hs) so it's directly unit-testable without spawning a process
-- or driving a real Parser.
-- ---------------------------------------------------------------------------

modeFlagIds :: [String]
modeFlagIds = ["borg", "dead", "greedy", "paranoid", "stoned", "tired", "wired", "youthful"]

isListRequested :: Map String JsonValue -> Bool
isListRequested flags = case Map.lookup "list" flags of
  Just (JsonBool True) -> True
  _ -> False

-- | Cow file basenames under @cowsDir@, sorted ordinally.
listCowFiles :: FilePath -> IO [String]
listCowFiles cowsDir = do
  entries <- listDirectory cowsDir
  let names = [dropExtension e | e <- entries, takeExtension e == ".cow"]
  pure (sort names)

-- | Resolves the message from the parsed \"message\" positional argument.
-- Returns 'Nothing' when no message was given on argv — the caller should
-- fall back to stdin.
resolveMessageFromArguments :: Map String JsonValue -> Maybe String
resolveMessageFromArguments args = case Map.lookup "message" args of
  Just (JsonArray items@(_ : _)) -> Just (intercalate " " (map jsonToString items))
  _ -> Nothing

jsonToString :: JsonValue -> String
jsonToString (JsonString s) = s
jsonToString JsonNull = ""
jsonToString (JsonNumber n) = show n
jsonToString (JsonBool b) = if b then "true" else "false"
jsonToString (JsonArray _) = ""
jsonToString (JsonObject _) = ""

-- | Builds a 'CowsayInvocation' from a resolved message and the parsed
-- flags map, applying cowsay.json's documented defaults for any flag that
-- wasn't explicitly set.
buildInvocation :: String -> Map String JsonValue -> CowsayInvocation
buildInvocation msg flags =
  CowsayInvocation
    { ciMessage = msg
    , ciEyes = getString "eyes" "oo"
    , ciTongue = getString "tongue" "  "
    , ciActiveModes = [m | m <- modeFlagIds, getBool m]
    , ciNoWrap = getBool "nowrap"
    , ciWidth = width
    , ciThink = getBool "think"
    , ciCowFile = getString "cowfile" "default"
    }
  where
    getString k defaultValue = case Map.lookup k flags of
      Just (JsonString s) -> s
      _ -> defaultValue
    getBool k = case Map.lookup k flags of
      Just (JsonBool True) -> True
      _ -> False
    width = case Map.lookup "width" flags of
      Just (JsonNumber n) -> clampWidth n
      _ -> 40
    clampWidth n
      | isNaN n = 40
      | n < 1 = 1
      | n > 2147483647 = 2147483647
      | otherwise = round n
