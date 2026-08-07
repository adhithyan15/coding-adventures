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
  , version
  ) where

import Data.Aeson (Value, toJSON)
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
  deriving (Eq, Show)

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
