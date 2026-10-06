-- | Pure geometry for linear barcodes.
--
-- Symbology packages turn their input into alternating 'Barcode1DRun'
-- values. This module validates that run stream, computes quiet-zone and
-- symbol geometry, and translates bars into the shared paint IR.
module CodingAdventures.BarcodeLayout1D
  ( Barcode1DRunColor (..)
  , Barcode1DRunRole (..)
  , Barcode1DSymbolRole (..)
  , Barcode1DRun (..)
  , Barcode1DSymbolLayout (..)
  , Barcode1DSymbolDescriptor (..)
  , Barcode1DLayout (..)
  , Barcode1DRenderConfig (..)
  , PaintBarcode1DOptions (..)
  , RunsFromBinaryPatternOptions (..)
  , RunsFromWidthPatternOptions (..)
  , Barcode1DError (..)
  , Barcode1DV1Error (..)
  , Barcode1DV1RenderConfig (..)
  , Barcode1DV1Options (..)
  , defaultBarcode1DRenderConfig
  , defaultPaintBarcode1DOptions
  , defaultBinaryPatternOptions
  , defaultWidthPatternOptions
  , totalModules
  , computeBarcode1DLayout
  , runsFromBinaryPattern
  , runsFromWidthPattern
  , layoutBarcode1D
  , drawBarcode1D
  , runsFromBinaryPatternV1
  , runsFromWidthPatternV1
  , computeBarcode1DLayoutV1
  , projectBarcode1DSceneV1
  , barcode1DErrorId
  , barcode1DV1ErrorId
  , version
  ) where

import Data.Aeson (Value, toJSON)
import Data.Char (ord)
import Data.List (find, group)
import Data.Map.Strict (Map)
import qualified Data.Map.Strict as Map
import Data.Maybe (isJust)
import CodingAdventures.PaintInstructions
  ( PaintInstruction (..)
  , PaintScene (..)
  )

-- | Package version shared with the established implementations.
version :: String
version = "0.1.0"

-- | Whether a run paints ink or advances through empty space.
data Barcode1DRunColor = Bar | Space
  deriving (Eq, Show)

-- | The semantic role of a run in its source symbology.
data Barcode1DRunRole
  = Data
  | Start
  | Stop
  | Guard
  | Check
  | InterCharacterGap
  deriving (Eq, Show)

-- | Symbol roles exclude gaps because gaps do not represent symbols.
data Barcode1DSymbolRole
  = SymbolData
  | SymbolStart
  | SymbolStop
  | SymbolGuard
  | SymbolCheck
  deriving (Eq, Show)

-- | One alternating bar or space measured in barcode modules.
data Barcode1DRun = Barcode1DRun
  { runColor :: Barcode1DRunColor
  , runModules :: Int
  , runSourceLabel :: String
  , runSourceIndex :: Int
  , runRole :: Barcode1DRunRole
  } deriving (Eq, Show)

-- | Inclusive/exclusive module span occupied by one encoded symbol.
data Barcode1DSymbolLayout = Barcode1DSymbolLayout
  { symbolLayoutLabel :: String
  , symbolLayoutStartModule :: Int
  , symbolLayoutEndModule :: Int
  , symbolLayoutSourceIndex :: Int
  , symbolLayoutRole :: Barcode1DSymbolRole
  } deriving (Eq, Show)

-- | Explicit symbol width supplied when run labels cannot infer boundaries.
data Barcode1DSymbolDescriptor = Barcode1DSymbolDescriptor
  { descriptorLabel :: String
  , descriptorModules :: Int
  , descriptorSourceIndex :: Int
  , descriptorRole :: Barcode1DSymbolRole
  } deriving (Eq, Show)

-- | Complete module-space geometry for a barcode.
data Barcode1DLayout = Barcode1DLayout
  { layoutLeftQuietZoneModules :: Int
  , layoutRightQuietZoneModules :: Int
  , layoutContentModules :: Int
  , layoutTotalModules :: Int
  , layoutSymbolLayouts :: [Barcode1DSymbolLayout]
  } deriving (Eq, Show)

-- | Geometry and paint choices. These never change encoded data.
data Barcode1DRenderConfig = Barcode1DRenderConfig
  { configModuleWidth :: Double
  , configBarHeight :: Double
  , configQuietZoneModules :: Int
  , configIncludeHumanReadableText :: Bool
  , configTextFontSize :: Double
  , configTextMargin :: Double
  , configForeground :: String
  , configBackground :: String
  } deriving (Eq, Show)

-- | Optional scene annotations and explicit symbol spans.
data PaintBarcode1DOptions = PaintBarcode1DOptions
  { optionsRenderConfig :: Barcode1DRenderConfig
  , optionsHumanReadableText :: Maybe String
  , optionsMetadata :: Map String Value
  , optionsLabel :: Maybe String
  , optionsSymbols :: Maybe [Barcode1DSymbolDescriptor]
  } deriving (Eq, Show)

-- | Source attribution attached to a binary-pattern run stream.
data RunsFromBinaryPatternOptions = RunsFromBinaryPatternOptions
  { binarySourceLabel :: String
  , binarySourceIndex :: Int
  , binaryRole :: Barcode1DRunRole
  } deriving (Eq, Show)

-- | Width-pattern markers, ratios, and source attribution.
data RunsFromWidthPatternOptions = RunsFromWidthPatternOptions
  { widthSourceLabel :: String
  , widthSourceIndex :: Int
  , widthRole :: Barcode1DRunRole
  , widthNarrowModules :: Int
  , widthWideModules :: Int
  , widthNarrowMarker :: Char
  , widthWideMarker :: Char
  , widthStartingColor :: Barcode1DRunColor
  } deriving (Eq, Show)

-- | Checked failures returned by the pure API.
data Barcode1DError
  = EmptyPattern String
  | UnsupportedBinaryToken Char
  | UnsupportedWidthToken Char
  | InvalidModuleCount String Int
  | NonAlternatingRuns Int
  | InvalidQuietZoneModules Int
  | SymbolWidthMismatch Int Int
  | InvalidRenderConfiguration String Double
  | HumanReadableTextUnsupported
  | PatternTooLong
  | InvalidMarkerConfiguration
  | TooManyRuns
  | ContentTooWide
  | InvalidSourceAttribution
  | TooManySymbols
  | MetadataTooLarge
  deriving (Eq, Show)

-- | Payload-blind, closed failures returned by the portable-v1 API.
--
-- The historical API above keeps its diagnostic payloads. Portable callers
-- receive only one of the contract's stable error identities, so unsupported
-- tokens, indices, and numeric values cannot leak through the error surface.
data Barcode1DV1Error
  = V1PatternTooLong
  | V1EmptyPattern
  | V1InvalidBinaryToken
  | V1InvalidWidthToken
  | V1InvalidMarkerConfiguration
  | V1InvalidModuleCount
  | V1InvalidSourceAttribution
  | V1TooManyRuns
  | V1ContentTooWide
  | V1NonAlternatingRuns
  | V1InvalidQuietZone
  | V1TooManySymbols
  | V1SymbolWidthMismatch
  | V1InvalidRenderConfig
  | V1MetadataTooLarge
  | V1HumanReadableTextUnsupported
  deriving (Eq, Show)

-- | Integer-only configuration for the language-neutral v1 projection.
data Barcode1DV1RenderConfig = Barcode1DV1RenderConfig
  { v1ModuleWidth :: Int
  , v1BarHeight :: Int
  , v1Foreground :: String
  , v1Background :: String
  , v1IncludeHumanReadableText :: Bool
  } deriving (Eq, Show)

-- | Closed options for the language-neutral v1 projection.
data Barcode1DV1Options = Barcode1DV1Options
  { v1RenderConfig :: Barcode1DV1RenderConfig
  , v1Label :: String
  , v1Metadata :: Map String String
  , v1HumanReadableText :: Maybe String
  , v1Symbols :: Maybe [Barcode1DSymbolDescriptor]
  } deriving (Eq, Show)

-- | Shared rendering defaults from the barcode contract.
defaultBarcode1DRenderConfig :: Barcode1DRenderConfig
defaultBarcode1DRenderConfig = Barcode1DRenderConfig
  { configModuleWidth = 4
  , configBarHeight = 120
  , configQuietZoneModules = 10
  , configIncludeHumanReadableText = False
  , configTextFontSize = 16
  , configTextMargin = 8
  , configForeground = "#000000"
  , configBackground = "#ffffff"
  }

-- | Default scene options with no caller metadata or explicit symbols.
defaultPaintBarcode1DOptions :: PaintBarcode1DOptions
defaultPaintBarcode1DOptions = PaintBarcode1DOptions
  { optionsRenderConfig = defaultBarcode1DRenderConfig
  , optionsHumanReadableText = Nothing
  , optionsMetadata = Map.empty
  , optionsLabel = Nothing
  , optionsSymbols = Nothing
  }

-- | Build the required attribution for a binary pattern.
defaultBinaryPatternOptions
  :: String -> Int -> Barcode1DRunRole -> RunsFromBinaryPatternOptions
defaultBinaryPatternOptions label index role = RunsFromBinaryPatternOptions
  { binarySourceLabel = label
  , binarySourceIndex = index
  , binaryRole = role
  }

-- | Build the standard 1:3 narrow/wide pattern configuration.
defaultWidthPatternOptions
  :: String -> Int -> Barcode1DRunRole -> RunsFromWidthPatternOptions
defaultWidthPatternOptions label index role = RunsFromWidthPatternOptions
  { widthSourceLabel = label
  , widthSourceIndex = index
  , widthRole = role
  , widthNarrowModules = 1
  , widthWideModules = 3
  , widthNarrowMarker = 'N'
  , widthWideMarker = 'W'
  , widthStartingColor = Bar
  }

-- | Add the module widths in a run stream.
totalModules :: [Barcode1DRun] -> Int
totalModules = sum . map runModules

-- | Coalesce a string of @0@ and @1@ modules into alternating runs.
runsFromBinaryPattern
  :: String
  -> RunsFromBinaryPatternOptions
  -> Either Barcode1DError [Barcode1DRun]
runsFromBinaryPattern patternText options
  | null patternText = Left (EmptyPattern "binary")
  | Just token <- find (`notElem` "01") patternText =
      Left (UnsupportedBinaryToken token)
  | otherwise = Right (map makeRun (group patternText))
  where
    makeRun tokens = Barcode1DRun
      { runColor = if head tokens == '1' then Bar else Space
      , runModules = length tokens
      , runSourceLabel = binarySourceLabel options
      , runSourceIndex = binarySourceIndex options
      , runRole = binaryRole options
      }

-- | Expand narrow/wide markers into alternating bar and space runs.
runsFromWidthPattern
  :: String
  -> RunsFromWidthPatternOptions
  -> Either Barcode1DError [Barcode1DRun]
runsFromWidthPattern patternText options
  | null patternText = Left (EmptyPattern "width")
  | widthNarrowModules options <= 0 =
      Left (InvalidModuleCount "narrowModules" (widthNarrowModules options))
  | widthWideModules options <= 0 =
      Left (InvalidModuleCount "wideModules" (widthWideModules options))
  | Just token <- find unsupported patternText = Left (UnsupportedWidthToken token)
  | otherwise = Right (zipWith makeRun [0 :: Int ..] patternText)
  where
    unsupported token =
      token /= widthNarrowMarker options && token /= widthWideMarker options
    makeRun index token = Barcode1DRun
      { runColor = colorAt index (widthStartingColor options)
      , runModules = if token == widthNarrowMarker options
          then widthNarrowModules options
          else widthWideModules options
      , runSourceLabel = widthSourceLabel options
      , runSourceIndex = widthSourceIndex options
      , runRole = widthRole options
      }

colorAt :: Int -> Barcode1DRunColor -> Barcode1DRunColor
colorAt index initial
  | even index = initial
  | initial == Bar = Space
  | otherwise = Bar

-- | Validate a run stream and compute quiet zones and symbol spans.
computeBarcode1DLayout
  :: [Barcode1DRun]
  -> Int
  -> Maybe [Barcode1DSymbolDescriptor]
  -> Either Barcode1DError Barcode1DLayout
computeBarcode1DLayout runs quietZone descriptors = do
  validateRuns runs
  if quietZone <= 0
    then Left (InvalidQuietZoneModules quietZone)
    else do
      let content = totalModules runs
      symbolLayouts <- case descriptors of
        Just values -> explicitSymbolLayouts values content
        Nothing -> Right (inferSymbolLayouts runs)
      Right (Barcode1DLayout
        { layoutLeftQuietZoneModules = quietZone
        , layoutRightQuietZoneModules = quietZone
        , layoutContentModules = content
        , layoutTotalModules = quietZone + content + quietZone
        , layoutSymbolLayouts = symbolLayouts
        })

validateRuns :: [Barcode1DRun] -> Either Barcode1DError ()
validateRuns = go 0 Nothing
  where
    go _ _ [] = Right ()
    go index previousColor (run : rest)
      | runModules run <= 0 =
          Left (InvalidModuleCount ("runs[" ++ show index ++ "].modules") (runModules run))
      | previousColor == Just (runColor run) = Left (NonAlternatingRuns index)
      | otherwise = go (index + 1) (Just (runColor run)) rest

explicitSymbolLayouts
  :: [Barcode1DSymbolDescriptor]
  -> Int
  -> Either Barcode1DError [Barcode1DSymbolLayout]
explicitSymbolLayouts descriptors content = go 0 [] descriptors
  where
    go cursor layouts []
      | cursor == content = Right (reverse layouts)
      | otherwise = Left (SymbolWidthMismatch content cursor)
    go cursor layouts (descriptor : rest)
      | descriptorModules descriptor <= 0 =
          Left (InvalidModuleCount
            ("symbol " ++ show (descriptorLabel descriptor) ++ " modules")
            (descriptorModules descriptor))
      | otherwise =
          let next = cursor + descriptorModules descriptor
              layout = Barcode1DSymbolLayout
                { symbolLayoutLabel = descriptorLabel descriptor
                , symbolLayoutStartModule = cursor
                , symbolLayoutEndModule = next
                , symbolLayoutSourceIndex = descriptorSourceIndex descriptor
                , symbolLayoutRole = descriptorRole descriptor
                }
          in go next (layout : layouts) rest

inferSymbolLayouts :: [Barcode1DRun] -> [Barcode1DSymbolLayout]
inferSymbolLayouts runs = reverse (finish finalCursor current layouts)
  where
    (finalCursor, current, layouts) = foldl step (0, Nothing, []) runs

    step (cursor, active, completed) run =
      let nextCursor = cursor + runModules run
      in case symbolRoleFromRunRole (runRole run) of
          Nothing -> (nextCursor, active, completed)
          Just role
            | sameSymbol active run role -> (nextCursor, active, completed)
            | otherwise ->
                ( nextCursor
                , Just (runSourceLabel run, cursor, runSourceIndex run, role)
                , finish cursor active completed
                )

    finish _ Nothing completed = completed
    finish cursor (Just (label, startModule, sourceIndex, role)) completed =
      Barcode1DSymbolLayout
        { symbolLayoutLabel = label
        , symbolLayoutStartModule = startModule
        , symbolLayoutEndModule = cursor
        , symbolLayoutSourceIndex = sourceIndex
        , symbolLayoutRole = role
        } : completed

sameSymbol
  :: Maybe (String, Int, Int, Barcode1DSymbolRole)
  -> Barcode1DRun
  -> Barcode1DSymbolRole
  -> Bool
sameSymbol Nothing _ _ = False
sameSymbol (Just (label, _, sourceIndex, role)) run candidateRole =
  label == runSourceLabel run
    && sourceIndex == runSourceIndex run
    && role == candidateRole

symbolRoleFromRunRole :: Barcode1DRunRole -> Maybe Barcode1DSymbolRole
symbolRoleFromRunRole role = case role of
  Data -> Just SymbolData
  Start -> Just SymbolStart
  Stop -> Just SymbolStop
  Guard -> Just SymbolGuard
  Check -> Just SymbolCheck
  InterCharacterGap -> Nothing

runRoleName :: Barcode1DRunRole -> String
runRoleName role = case role of
  Data -> "data"
  Start -> "start"
  Stop -> "stop"
  Guard -> "guard"
  Check -> "check"
  InterCharacterGap -> "inter-character-gap"

-- | Translate barcode runs to rectangle-only paint instructions.
layoutBarcode1D
  :: [Barcode1DRun]
  -> PaintBarcode1DOptions
  -> Either Barcode1DError PaintScene
layoutBarcode1D runs options = do
  validateRenderConfig config
  if configIncludeHumanReadableText config || isJust (optionsHumanReadableText options)
    then Left HumanReadableTextUnsupported
    else do
      layout <- computeBarcode1DLayout
        runs
        (configQuietZoneModules config)
        (optionsSymbols options)
      let instructions = renderRuns config (layoutLeftQuietZoneModules layout) runs
          sceneWidth = fromIntegral (layoutTotalModules layout) * configModuleWidth config
          sceneHeight = configBarHeight config
          sceneMetadata = Map.union
            (standardMetadata options layout sceneWidth sceneHeight)
            (optionsMetadata options)
      Right PaintScene
        { psWidth = sceneWidth
        , psHeight = sceneHeight
        , psInstructions = instructions
        , psBg = configBackground config
        , psMeta = sceneMetadata
        }
  where
    config = optionsRenderConfig options

-- | Alias matching the shared public API name.
drawBarcode1D
  :: [Barcode1DRun]
  -> PaintBarcode1DOptions
  -> Either Barcode1DError PaintScene
drawBarcode1D = layoutBarcode1D

validateRenderConfig :: Barcode1DRenderConfig -> Either Barcode1DError ()
validateRenderConfig config = do
  validatePositive "moduleWidth" (configModuleWidth config)
  validatePositive "barHeight" (configBarHeight config)
  if configQuietZoneModules config <= 0
    then Left (InvalidQuietZoneModules (configQuietZoneModules config))
    else Right ()
  validatePositive "textFontSize" (configTextFontSize config)
  validateNonNegative "textMargin" (configTextMargin config)

validatePositive :: String -> Double -> Either Barcode1DError ()
validatePositive name value
  | finite value && value > 0 = Right ()
  | otherwise = Left (InvalidRenderConfiguration name value)

validateNonNegative :: String -> Double -> Either Barcode1DError ()
validateNonNegative name value
  | finite value && value >= 0 = Right ()
  | otherwise = Left (InvalidRenderConfiguration name value)

finite :: Double -> Bool
finite value = not (isNaN value || isInfinite value)

renderRuns
  :: Barcode1DRenderConfig
  -> Int
  -> [Barcode1DRun]
  -> [PaintInstruction]
renderRuns config quietZone = reverse . snd . foldl renderRun (quietZone, [])
  where
    renderRun (cursor, instructions) run =
      let next = cursor + runModules run
          instruction = PaintRect
            { prX = fromIntegral cursor * configModuleWidth config
            , prY = 0
            , prW = fromIntegral (runModules run) * configModuleWidth config
            , prH = configBarHeight config
            , prFill = configForeground config
            , prStroke = ""
            , prStrokeWidth = 0
            , prMeta = Map.fromList
                [ ("sourceLabel", toJSON (runSourceLabel run))
                , ("sourceIndex", toJSON (runSourceIndex run))
                , ("role", toJSON (runRoleName (runRole run)))
                , ("moduleStart", toJSON cursor)
                , ("moduleEnd", toJSON next)
                ]
            }
      in if runColor run == Bar
          then (next, instruction : instructions)
          else (next, instructions)

standardMetadata
  :: PaintBarcode1DOptions
  -> Barcode1DLayout
  -> Double
  -> Double
  -> Map String Value
standardMetadata options layout sceneWidth sceneHeight = Map.fromList
  [ ("label", toJSON (maybe "1D barcode" id (optionsLabel options)))
  , ("leftQuietZoneModules", toJSON (layoutLeftQuietZoneModules layout))
  , ("rightQuietZoneModules", toJSON (layoutRightQuietZoneModules layout))
  , ("contentModules", toJSON (layoutContentModules layout))
  , ("totalModules", toJSON (layoutTotalModules layout))
  , ("moduleWidthPx", toJSON (configModuleWidth (optionsRenderConfig options)))
  , ("barHeightPx", toJSON (configBarHeight (optionsRenderConfig options)))
  , ("sceneWidthPx", toJSON sceneWidth)
  , ("sceneHeightPx", toJSON sceneHeight)
  , ("symbolCount", toJSON (length (layoutSymbolLayouts layout)))
  ]

-- Strict language-neutral v1 adapter.  The historical functions above retain
-- their established types and behaviour for the symbology packages.

maxPatternScalars, maxRuns, maxContentModules, maxQuietZoneModules :: Int
maxPatternScalars = 65567
maxRuns = 40979
maxContentModules = 65567
maxQuietZoneModules = 4096

maxSymbols, maxLabelScalars, maxMetadataEntries, maxMetadataKeyScalars :: Int
maxSymbols = 40979
maxLabelScalars = 4096
maxMetadataEntries = 64
maxMetadataKeyScalars = 128

maxMetadataValueScalars, maxMetadataUtf8Bytes, maxModuleWidth :: Int
maxMetadataValueScalars = 4096
maxMetadataUtf8Bytes = 65536
maxModuleWidth = 8192

maxBarHeight, maxColorScalars :: Int
maxBarHeight = 8192
maxColorScalars = 128

barcode1DErrorId :: Barcode1DError -> String
barcode1DErrorId err = case err of
  PatternTooLong -> "pattern-too-long"
  EmptyPattern _ -> "empty-pattern"
  UnsupportedBinaryToken _ -> "invalid-binary-token"
  UnsupportedWidthToken _ -> "invalid-width-token"
  InvalidMarkerConfiguration -> "invalid-marker-configuration"
  InvalidModuleCount _ _ -> "invalid-module-count"
  InvalidSourceAttribution -> "invalid-source-attribution"
  TooManyRuns -> "too-many-runs"
  ContentTooWide -> "content-too-wide"
  NonAlternatingRuns _ -> "non-alternating-runs"
  InvalidQuietZoneModules _ -> "invalid-quiet-zone"
  TooManySymbols -> "too-many-symbols"
  SymbolWidthMismatch _ _ -> "symbol-width-mismatch"
  InvalidRenderConfiguration _ _ -> "invalid-render-config"
  MetadataTooLarge -> "metadata-too-large"
  HumanReadableTextUnsupported -> "human-readable-text-unsupported"

-- | Stable identifier for a closed portable-v1 failure.
barcode1DV1ErrorId :: Barcode1DV1Error -> String
barcode1DV1ErrorId err = case err of
  V1PatternTooLong -> "pattern-too-long"
  V1EmptyPattern -> "empty-pattern"
  V1InvalidBinaryToken -> "invalid-binary-token"
  V1InvalidWidthToken -> "invalid-width-token"
  V1InvalidMarkerConfiguration -> "invalid-marker-configuration"
  V1InvalidModuleCount -> "invalid-module-count"
  V1InvalidSourceAttribution -> "invalid-source-attribution"
  V1TooManyRuns -> "too-many-runs"
  V1ContentTooWide -> "content-too-wide"
  V1NonAlternatingRuns -> "non-alternating-runs"
  V1InvalidQuietZone -> "invalid-quiet-zone"
  V1TooManySymbols -> "too-many-symbols"
  V1SymbolWidthMismatch -> "symbol-width-mismatch"
  V1InvalidRenderConfig -> "invalid-render-config"
  V1MetadataTooLarge -> "metadata-too-large"
  V1HumanReadableTextUnsupported -> "human-readable-text-unsupported"

validScalar :: Char -> Bool
validScalar value = let point = ord value in point < 0xd800 || point > 0xdfff

validString :: Int -> String -> Bool
validString limit value = length value <= limit && all validScalar value

validSource :: String -> Int -> Bool
validSource label index =
  validString maxLabelScalars label
    && toInteger index >= (-2147483648)
    && toInteger index <= 2147483647

checkedAddContent :: Int -> Int -> Either Barcode1DV1Error Int
checkedAddContent current addition
  | addition <= 0 = Left V1InvalidModuleCount
  | addition > maxContentModules - current = Left V1ContentTooWide
  | otherwise = Right (current + addition)

runsFromBinaryPatternV1
  :: String
  -> RunsFromBinaryPatternOptions
  -> Either Barcode1DV1Error [Barcode1DRun]
runsFromBinaryPatternV1 patternText options
  | length patternText > maxPatternScalars = Left V1PatternTooLong
  | null patternText = Left V1EmptyPattern
  | Just _ <- find (`notElem` "01") patternText = Left V1InvalidBinaryToken
  | not (validSource (binarySourceLabel options) (binarySourceIndex options)) =
      Left V1InvalidSourceAttribution
  | otherwise = emitRuns (head patternText) 1 0 [] (tail patternText)
  where
    makeRun token modules = Barcode1DRun
      { runColor = if token == '1' then Bar else Space
      , runModules = modules
      , runSourceLabel = binarySourceLabel options
      , runSourceIndex = binarySourceIndex options
      , runRole = binaryRole options
      }
    emitRuns token modules emitted result []
      | emitted >= maxRuns = Left V1TooManyRuns
      | otherwise = Right (reverse (makeRun token modules : result))
    emitRuns token modules emitted result (next : rest)
      | next == token = emitRuns token (modules + 1) emitted result rest
      | emitted >= maxRuns = Left V1TooManyRuns
      | otherwise = emitRuns next 1 (emitted + 1)
          (makeRun token modules : result) rest

runsFromWidthPatternV1
  :: String
  -> RunsFromWidthPatternOptions
  -> Either Barcode1DV1Error [Barcode1DRun]
runsFromWidthPatternV1 patternText options
  | length patternText > maxPatternScalars = Left V1PatternTooLong
  | null patternText = Left V1EmptyPattern
  | not (validScalar (widthNarrowMarker options))
      || not (validScalar (widthWideMarker options))
      || widthNarrowMarker options == widthWideMarker options =
          Left V1InvalidMarkerConfiguration
  | Just _ <- find unsupported patternText = Left V1InvalidWidthToken
  | not (validSource (widthSourceLabel options) (widthSourceIndex options)) =
      Left V1InvalidSourceAttribution
  | widthNarrowModules options <= 0 =
      Left V1InvalidModuleCount
  | widthWideModules options <= 0 =
      Left V1InvalidModuleCount
  | length patternText > maxRuns = Left V1TooManyRuns
  | otherwise = go 0 0 [] patternText
  where
    unsupported token =
      token /= widthNarrowMarker options && token /= widthWideMarker options
    go _ _ result [] = Right (reverse result)
    go index content result (token : rest) = do
      let modules = if token == widthNarrowMarker options
            then widthNarrowModules options else widthWideModules options
      next <- checkedAddContent content modules
      let run = Barcode1DRun
            { runColor = colorAt index (widthStartingColor options)
            , runModules = modules
            , runSourceLabel = widthSourceLabel options
            , runSourceIndex = widthSourceIndex options
            , runRole = widthRole options
            }
      go (index + 1) next (run : result) rest

computeBarcode1DLayoutV1
  :: [Barcode1DRun]
  -> Int
  -> Maybe [Barcode1DSymbolDescriptor]
  -> Either Barcode1DV1Error Barcode1DLayout
computeBarcode1DLayoutV1 runs quiet descriptors = do
  content <- validateV1Runs runs
  if quiet < 1 || quiet > maxQuietZoneModules
    then Left V1InvalidQuietZone
    else do
      layouts <- case descriptors of
        Just values -> explicitV1Layouts values content
        Nothing -> inferV1Layouts runs
      Right Barcode1DLayout
        { layoutLeftQuietZoneModules = quiet
        , layoutRightQuietZoneModules = quiet
        , layoutContentModules = content
        , layoutTotalModules = quiet + content + quiet
        , layoutSymbolLayouts = layouts
        }

validateV1Runs :: [Barcode1DRun] -> Either Barcode1DV1Error Int
validateV1Runs runs
  | length runs > maxRuns = Left V1TooManyRuns
  | otherwise = go Nothing 0 runs
  where
    go _ content [] = Right content
    go previous content (run : rest)
      | runModules run <= 0 =
          Left V1InvalidModuleCount
      | not (validSource (runSourceLabel run) (runSourceIndex run)) =
          Left V1InvalidSourceAttribution
      | runModules run > maxContentModules - content = Left V1ContentTooWide
      | previous == Just (runColor run) = Left V1NonAlternatingRuns
      | otherwise = do
          next <- checkedAddContent content (runModules run)
          go (Just (runColor run)) next rest

explicitV1Layouts
  :: [Barcode1DSymbolDescriptor]
  -> Int
  -> Either Barcode1DV1Error [Barcode1DSymbolLayout]
explicitV1Layouts descriptors content
  | length descriptors > maxSymbols = Left V1TooManySymbols
  | otherwise = go 0 [] descriptors
  where
    go cursor result []
      | cursor == content = Right (reverse result)
      | otherwise = Left V1SymbolWidthMismatch
    go cursor result (descriptor : rest)
      | descriptorModules descriptor <= 0 =
          Left V1InvalidModuleCount
      | not (validSource (descriptorLabel descriptor) (descriptorSourceIndex descriptor)) =
          Left V1InvalidSourceAttribution
      | descriptorModules descriptor > maxContentModules - cursor =
          Left V1SymbolWidthMismatch
      | otherwise =
          let next = cursor + descriptorModules descriptor
              layout = Barcode1DSymbolLayout
                (descriptorLabel descriptor) cursor next
                (descriptorSourceIndex descriptor) (descriptorRole descriptor)
          in go next (layout : result) rest

inferV1Layouts :: [Barcode1DRun] -> Either Barcode1DV1Error [Barcode1DSymbolLayout]
inferV1Layouts runs =
  let layouts = inferSymbolLayouts runs
  in if length layouts > maxSymbols then Left V1TooManySymbols else Right layouts

utf8Length :: String -> Int
utf8Length = sum . map width
  where
    width value
      | ord value <= 0x7f = 1
      | ord value <= 0x7ff = 2
      | ord value <= 0xffff = 3
      | otherwise = 4

validateV1Metadata :: Map String String -> Either Barcode1DV1Error ()
validateV1Metadata metadata
  | Map.size metadata > maxMetadataEntries = Left V1MetadataTooLarge
  | any invalidEntry (Map.toList metadata) = Left V1MetadataTooLarge
  | sum (map encodedSize (Map.toList metadata)) > maxMetadataUtf8Bytes =
      Left V1MetadataTooLarge
  | otherwise = Right ()
  where
    invalidEntry (key, value) =
      not (validString maxMetadataKeyScalars key)
        || not (validString maxMetadataValueScalars value)
    encodedSize (key, value) = utf8Length key + utf8Length value

projectBarcode1DSceneV1
  :: [Barcode1DRun]
  -> Int
  -> Barcode1DV1Options
  -> Either Barcode1DV1Error PaintScene
projectBarcode1DSceneV1 runs quiet options
  | v1IncludeHumanReadableText config || isJust (v1HumanReadableText options) =
      Left V1HumanReadableTextUnsupported
  | v1ModuleWidth config < 1 || v1ModuleWidth config > maxModuleWidth =
      Left V1InvalidRenderConfig
  | v1BarHeight config < 1 || v1BarHeight config > maxBarHeight =
      Left V1InvalidRenderConfig
  | not (validString maxColorScalars (v1Foreground config)) =
      Left V1InvalidRenderConfig
  | not (validString maxColorScalars (v1Background config)) =
      Left V1InvalidRenderConfig
  | otherwise = do
      layout <- computeBarcode1DLayoutV1 runs quiet (v1Symbols options)
      validateV1Metadata (v1Metadata options)
      if not (validString maxLabelScalars (v1Label options))
        then Left V1MetadataTooLarge
        else
          let sceneWidth = layoutTotalModules layout * v1ModuleWidth config
              canonical = Map.fromList
                [ ("label", v1Label options)
                , ("leftQuietZoneModules", show (layoutLeftQuietZoneModules layout))
                , ("rightQuietZoneModules", show (layoutRightQuietZoneModules layout))
                , ("contentModules", show (layoutContentModules layout))
                , ("totalModules", show (layoutTotalModules layout))
                , ("moduleWidthPx", show (v1ModuleWidth config))
                , ("barHeightPx", show (v1BarHeight config))
                , ("sceneWidthPx", show sceneWidth)
                , ("sceneHeightPx", show (v1BarHeight config))
                , ("symbolCount", show (length (layoutSymbolLayouts layout)))
                ]
          in Right PaintScene
            { psWidth = fromIntegral sceneWidth
            , psHeight = fromIntegral (v1BarHeight config)
            , psInstructions = v1RenderRuns config quiet runs
            , psBg = v1Background config
            , psMeta = Map.map toJSON (Map.union canonical (v1Metadata options))
            }
  where
    config = v1RenderConfig options

v1RenderRuns :: Barcode1DV1RenderConfig -> Int -> [Barcode1DRun] -> [PaintInstruction]
v1RenderRuns config quiet = reverse . snd . foldl render (quiet, [])
  where
    render (cursor, result) run =
      let next = cursor + runModules run
          rectangle = PaintRect
            { prX = fromIntegral (cursor * v1ModuleWidth config)
            , prY = 0
            , prW = fromIntegral (runModules run * v1ModuleWidth config)
            , prH = fromIntegral (v1BarHeight config)
            , prFill = v1Foreground config
            , prStroke = ""
            , prStrokeWidth = 0
            , prMeta = Map.fromList
                [ ("sourceLabel", toJSON (runSourceLabel run))
                , ("sourceIndex", toJSON (show (runSourceIndex run)))
                , ("role", toJSON (runRoleName (runRole run)))
                , ("moduleStart", toJSON (show cursor))
                , ("moduleEnd", toJSON (show next))
                ]
            }
      in if runColor run == Bar
          then (next, rectangle : result)
          else (next, result)
