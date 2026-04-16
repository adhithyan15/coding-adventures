module CliBuilder
    ( JsonValue(..)
    , BuiltinFlags(..)
    , FlagDef(..)
    , ArgumentDef(..)
    , CommandDef(..)
    , ExclusiveGroup(..)
    , CliSpec(..)
    , ParseError(..)
    , ParseErrors(..)
    , CliBuilderError(..)
    , ParseResult(..)
    , HelpResult(..)
    , VersionResult(..)
    , ParserOutput(..)
    , ValidationResult(..)
    , FlagInfo(..)
    , TokenEvent(..)
    , Parser
    , description
    , loadSpecFromStr
    , loadSpecFromFile
    , validateSpecStr
    , validateSpecFile
    , generateRootHelp
    , generateCommandHelp
    , newParser
    , parseArgs
    , flagInfoFromDef
    , classifyToken
    , classifyTraditional
    ) where

import Control.Exception (IOException, try)
import qualified Data.Map.Strict as Map
import Data.Map.Strict (Map)
import qualified Data.Set as Set
import Data.Set (Set)
import Data.Char (isSpace)
import Data.List (find, intercalate, nub, tails)
import Data.Maybe (fromMaybe, isJust, mapMaybe)
import JsonSerializer (renderJson)
import JsonValue (JsonValue(..), parseJson)

description :: String
description = "Declarative CLI parsing from JSON specs"

data BuiltinFlags = BuiltinFlags
    { builtinHelp :: Bool
    , builtinVersion :: Bool
    }
    deriving (Eq, Show)

data FlagDef = FlagDef
    { flagId :: String
    , flagShort :: Maybe String
    , flagLong :: Maybe String
    , flagSingleDashLong :: Maybe String
    , flagDescription :: String
    , flagType :: String
    , flagRequired :: Bool
    , flagDefault :: Maybe JsonValue
    , flagValueName :: Maybe String
    , flagEnumValues :: [String]
    , flagConflictsWith :: [String]
    , flagRequires :: [String]
    , flagRequiredUnless :: [String]
    , flagRepeatable :: Bool
    , flagDefaultWhenPresent :: Maybe String
    }
    deriving (Eq, Show)

data ArgumentDef = ArgumentDef
    { argumentId :: String
    , argumentDisplayName :: String
    , argumentDescription :: String
    , argumentType :: String
    , argumentRequired :: Bool
    , argumentVariadic :: Bool
    , argumentVariadicMin :: Int
    , argumentVariadicMax :: Maybe Int
    , argumentDefault :: Maybe JsonValue
    , argumentEnumValues :: [String]
    , argumentRequiredUnlessFlag :: [String]
    }
    deriving (Eq, Show)

data ExclusiveGroup = ExclusiveGroup
    { groupId :: String
    , groupFlagIds :: [String]
    , groupRequired :: Bool
    }
    deriving (Eq, Show)

data CommandDef = CommandDef
    { commandId :: String
    , commandName :: String
    , commandAliases :: [String]
    , commandDescription :: String
    , commandInheritGlobalFlags :: Bool
    , commandFlags :: [FlagDef]
    , commandArguments :: [ArgumentDef]
    , commandCommands :: [CommandDef]
    , commandExclusiveGroups :: [ExclusiveGroup]
    }
    deriving (Eq, Show)

data CliSpec = CliSpec
    { cliBuilderSpecVersion :: String
    , cliName :: String
    , cliDisplayName :: Maybe String
    , cliDescription :: String
    , cliVersion :: Maybe String
    , cliParsingMode :: String
    , cliBuiltinFlags :: BuiltinFlags
    , cliGlobalFlags :: [FlagDef]
    , cliFlags :: [FlagDef]
    , cliArguments :: [ArgumentDef]
    , cliCommands :: [CommandDef]
    , cliMutuallyExclusiveGroups :: [ExclusiveGroup]
    }
    deriving (Eq, Show)

data ParseError = ParseError
    { parseErrorType :: String
    , parseErrorMessage :: String
    , parseErrorSuggestion :: Maybe String
    , parseErrorContext :: [String]
    }
    deriving (Eq, Show)

newtype ParseErrors = ParseErrors
    { parseErrors :: [ParseError]
    }
    deriving (Eq, Show)

data CliBuilderError
    = SpecError String
    | ParseFailure ParseErrors
    | IoError String
    | JsonError String
    deriving (Eq, Show)

data ParseResult = ParseResult
    { resultProgram :: String
    , resultCommandPath :: [String]
    , resultFlags :: Map String JsonValue
    , resultArguments :: Map String JsonValue
    , resultExplicitFlags :: [String]
    }
    deriving (Eq, Show)

data HelpResult = HelpResult
    { helpText :: String
    , helpCommandPath :: [String]
    }
    deriving (Eq, Show)

newtype VersionResult = VersionResult
    { versionText :: String
    }
    deriving (Eq, Show)

data ParserOutput
    = ParseOutput ParseResult
    | HelpOutput HelpResult
    | VersionOutput VersionResult
    deriving (Eq, Show)

data ValidationResult
    = ValidationSuccess
    | ValidationFailure String
    deriving (Eq, Show)

data FlagInfo = FlagInfo
    { infoId :: String
    , infoShort :: Maybe Char
    , infoLong :: Maybe String
    , infoSingleDashLong :: Maybe String
    , infoBoolean :: Bool
    , infoCount :: Bool
    , infoHasDefaultWhenPresent :: Bool
    }
    deriving (Eq, Show)

data TokenEvent
    = EndOfFlags
    | LongFlag String
    | LongFlagWithValue String String
    | SingleDashLong String
    | ShortFlag Char
    | ShortFlagWithValue Char String
    | StackedFlags [Char]
    | Positional String
    | UnknownFlag String
    deriving (Eq, Show)

newtype Parser = Parser
    { parserSpec :: CliSpec
    }

loadSpecFromStr :: String -> Either CliBuilderError CliSpec
loadSpecFromStr input = do
    value <- either (Left . JsonError) Right (parseJson input)
    spec <- parseCliSpec value
    validateSpec spec
    Right spec

loadSpecFromFile :: FilePath -> IO (Either CliBuilderError CliSpec)
loadSpecFromFile path = do
    result <- try (readFile path) :: IO (Either IOException String)
    pure $
        case result of
            Left err -> Left (IoError (show err))
            Right contents -> loadSpecFromStr contents

validateSpecStr :: String -> ValidationResult
validateSpecStr input =
    case loadSpecFromStr input of
        Left err -> ValidationFailure (showCliBuilderError err)
        Right _ -> ValidationSuccess

validateSpecFile :: FilePath -> IO ValidationResult
validateSpecFile path = do
    result <- loadSpecFromFile path
    pure $
        case result of
            Left err -> ValidationFailure (showCliBuilderError err)
            Right _ -> ValidationSuccess

newParser :: CliSpec -> Parser
newParser = Parser

parseArgs :: Parser -> [String] -> Either CliBuilderError ParserOutput
parseArgs (Parser spec) args =
    case args of
        [] -> Left (SpecError "argv must have at least one element (the program name)")
        program : argv -> do
            let routing = routeCommand spec program argv
                commandPath = routedPath routing
                (baseFlags, activeArguments, activeGroups) =
                    activeScope spec commandPath
                scannerFlags = baseFlags ++ builtinFlagsFor spec
                classifierFlags = map flagInfoFromDef scannerFlags
                scanResult =
                    scanTokens
                        spec
                        scannerFlags
                        classifierFlags
                        commandPath
                        (routedCommandIndices routing)
                        argv
                validationErrors =
                    resolveArguments commandPath activeArguments (scannedPositionals scanResult) (scannedParsedFlags scanResult)
                flagErrors =
                    validateParsedFlags commandPath baseFlags activeGroups (scannedExplicitFlags scanResult)
                allErrors =
                    scannedErrors scanResult ++ fst validationErrors ++ flagErrors
             in if scannedHelpRequested scanResult
                    then Right
                        (HelpOutput
                            (HelpResult
                                (if length commandPath <= 1
                                    then generateRootHelp spec
                                    else generateCommandHelp spec commandPath
                                )
                                commandPath
                            )
                        )
                    else if scannedVersionRequested scanResult
                        then Right (VersionOutput (VersionResult (fromMaybe "" (cliVersion spec))))
                        else
                            case allErrors of
                                [] ->
                                    Right
                                        (ParseOutput
                                            (ParseResult
                                                program
                                                commandPath
                                                (populateFlagDefaults baseFlags (scannedParsedFlags scanResult))
                                                (snd validationErrors)
                                                (scannedExplicitFlags scanResult)
                                            )
                                        )
                                errs -> Left (ParseFailure (ParseErrors errs))

generateRootHelp :: CliSpec -> String
generateRootHelp spec =
    intercalate "\n" (filter (not . null) [usageSection, descriptionSection, commandsSection, optionsSection, argumentsSection, globalOptionsSection])
  where
    usageParts =
        [cliName spec]
            ++ if hasAnyOptions (cliFlags spec ++ cliGlobalFlags spec) (builtinFlagsFor spec)
                then ["[OPTIONS]"]
                else []
            ++ if null (cliCommands spec) then [] else ["[COMMAND]"]
            ++ map formatArgumentUsage (cliArguments spec)
    usageSection = unlines ["USAGE", "  " ++ unwords usageParts]
    descriptionSection = unlines ["DESCRIPTION", "  " ++ cliDescription spec]
    commandsSection = formatCommandsSection (cliCommands spec)
    optionsSection = formatOptionsSection "OPTIONS" (cliFlags spec)
    argumentsSection = formatArgumentsSection (cliArguments spec)
    globalOptionsSection = formatOptionsSection "GLOBAL OPTIONS" (cliGlobalFlags spec ++ builtinFlagsFor spec)

generateCommandHelp :: CliSpec -> [String] -> String
generateCommandHelp spec commandPath =
    case findCommandByPath spec (drop 1 commandPath) of
        Nothing -> generateRootHelp spec
        Just commandDef ->
            intercalate "\n" (filter (not . null) [usageSection, descriptionSection, commandsSection, optionsSection, argumentsSection, globalOptionsSection])
          where
            usageParts =
                [cliName spec]
                    ++ drop 1 commandPath
                    ++ if hasAnyOptions (commandFlags commandDef) []
                        then ["[OPTIONS]"]
                        else []
                    ++ if null (commandCommands commandDef) then [] else ["[COMMAND]"]
                    ++ map formatArgumentUsage (commandArguments commandDef)
            usageSection = unlines ["USAGE", "  " ++ unwords usageParts]
            descriptionSection = unlines ["DESCRIPTION", "  " ++ commandDescription commandDef]
            commandsSection = formatCommandsSection (commandCommands commandDef)
            optionsSection = formatOptionsSection "OPTIONS" (commandFlags commandDef)
            globalOptionsSection =
                if commandInheritGlobalFlags commandDef
                    then formatOptionsSection "GLOBAL OPTIONS" (cliGlobalFlags spec ++ builtinFlagsFor spec)
                    else ""
            argumentsSection = formatArgumentsSection (commandArguments commandDef)

classifyToken :: [FlagInfo] -> String -> [TokenEvent]
classifyToken flagInfos token
    | token == "--" = [EndOfFlags]
    | token == "-" = [Positional token]
    | Just rest <- stripPrefix "--" token = classifyLongToken flagInfos rest
    | Just rest <- stripPrefix "-" token, not (null rest) = classifyShortOrSingleDashLong flagInfos rest token
    | otherwise = [Positional token]

classifyTraditional :: [FlagInfo] -> String -> [String] -> [TokenEvent]
classifyTraditional flagInfos token knownSubcommands
    | token `elem` knownSubcommands = [Positional token]
    | all (`isKnownShort` flagInfos) token = [StackedFlags token]
    | otherwise = [Positional token]

parseCliSpec :: JsonValue -> Either CliBuilderError CliSpec
parseCliSpec value = do
    objectValue <- asObject "CLI spec" value
    builtinObject <- maybe (Right defaultBuiltins) parseBuiltinFlags (lookupField "builtin_flags" objectValue)
    parsingMode <- fromMaybe "gnu" <$> optionalString "parsing_mode" objectValue
    CliSpec
        <$> requireString "cli_builder_spec_version" objectValue
        <*> requireString "name" objectValue
        <*> optionalString "display_name" objectValue
        <*> requireString "description" objectValue
        <*> optionalString "version" objectValue
        <*> pure parsingMode
        <*> pure builtinObject
        <*> arrayFieldWithDefault "global_flags" parseFlagDef objectValue
        <*> arrayFieldWithDefault "flags" parseFlagDef objectValue
        <*> arrayFieldWithDefault "arguments" parseArgumentDef objectValue
        <*> arrayFieldWithDefault "commands" parseCommandDef objectValue
        <*> arrayFieldWithDefault "mutually_exclusive_groups" parseExclusiveGroup objectValue

parseBuiltinFlags :: JsonValue -> Either CliBuilderError BuiltinFlags
parseBuiltinFlags value = do
    objectValue <- asObject "builtin_flags" value
    helpEnabled <- fromMaybe True <$> optionalBool "help" objectValue
    versionEnabled <- fromMaybe True <$> optionalBool "version" objectValue
    BuiltinFlags
        <$> pure helpEnabled
        <*> pure versionEnabled

parseFlagDef :: JsonValue -> Either CliBuilderError FlagDef
parseFlagDef value = do
    objectValue <- asObject "flag" value
    requiredValue <- fromMaybe False <$> optionalBool "required" objectValue
    repeatableValue <- fromMaybe False <$> optionalBool "repeatable" objectValue
    FlagDef
        <$> requireString "id" objectValue
        <*> optionalString "short" objectValue
        <*> optionalString "long" objectValue
        <*> optionalString "single_dash_long" objectValue
        <*> requireString "description" objectValue
        <*> requireString "type" objectValue
        <*> pure requiredValue
        <*> optionalJson "default" objectValue
        <*> optionalString "value_name" objectValue
        <*> stringArrayField "enum_values" objectValue
        <*> stringArrayField "conflicts_with" objectValue
        <*> stringArrayField "requires" objectValue
        <*> stringArrayField "required_unless" objectValue
        <*> pure repeatableValue
        <*> optionalString "default_when_present" objectValue

parseArgumentDef :: JsonValue -> Either CliBuilderError ArgumentDef
parseArgumentDef value = do
    objectValue <- asObject "argument" value
    displayName <-
        case optionalString "display_name" objectValue of
            Right (Just nameValue) -> Right nameValue
            Right Nothing -> requireString "name" objectValue
            Left err -> Left err
    requiredValue <- fromMaybe True <$> optionalBool "required" objectValue
    variadicValue <- fromMaybe False <$> optionalBool "variadic" objectValue
    variadicMin <- fromMaybe 1 <$> optionalInt "variadic_min" objectValue
    ArgumentDef
        <$> requireString "id" objectValue
        <*> pure displayName
        <*> requireString "description" objectValue
        <*> requireString "type" objectValue
        <*> pure requiredValue
        <*> pure variadicValue
        <*> pure variadicMin
        <*> optionalInt "variadic_max" objectValue
        <*> optionalJson "default" objectValue
        <*> stringArrayField "enum_values" objectValue
        <*> stringArrayField "required_unless_flag" objectValue

parseCommandDef :: JsonValue -> Either CliBuilderError CommandDef
parseCommandDef value = do
    objectValue <- asObject "command" value
    inheritGlobals <- fromMaybe True <$> optionalBool "inherit_global_flags" objectValue
    CommandDef
        <$> requireString "id" objectValue
        <*> requireString "name" objectValue
        <*> stringArrayField "aliases" objectValue
        <*> requireString "description" objectValue
        <*> pure inheritGlobals
        <*> arrayFieldWithDefault "flags" parseFlagDef objectValue
        <*> arrayFieldWithDefault "arguments" parseArgumentDef objectValue
        <*> arrayFieldWithDefault "commands" parseCommandDef objectValue
        <*> arrayFieldWithDefault "mutually_exclusive_groups" parseExclusiveGroup objectValue

parseExclusiveGroup :: JsonValue -> Either CliBuilderError ExclusiveGroup
parseExclusiveGroup value = do
    objectValue <- asObject "exclusive group" value
    requiredValue <- fromMaybe False <$> optionalBool "required" objectValue
    ExclusiveGroup
        <$> requireString "id" objectValue
        <*> arrayField "flag_ids" requireJsonString objectValue
        <*> pure requiredValue

validateSpec :: CliSpec -> Either CliBuilderError ()
validateSpec spec = do
    if cliBuilderSpecVersion spec == "1.0"
        then Right ()
        else Left (SpecError "unsupported cli_builder_spec_version; only \"1.0\" is supported")
    validateFlagsSelfConsistency "global_flags" (cliGlobalFlags spec)
    validateScope
        ("root (" ++ cliName spec ++ ")")
        (cliGlobalFlags spec)
        (cliFlags spec)
        (cliArguments spec)
        (cliCommands spec)
        (cliMutuallyExclusiveGroups spec)
    mapM_ (validateCommand spec (map flagId (cliGlobalFlags spec))) (cliCommands spec)

validateCommand :: CliSpec -> [String] -> CommandDef -> Either CliBuilderError ()
validateCommand spec globalIds commandDef = do
    let inheritedGlobals =
            if commandInheritGlobalFlags commandDef
                then filter (\flagDef -> flagId flagDef `elem` globalIds) (cliGlobalFlags spec)
                else []
    validateScope
        ("command '" ++ commandName commandDef ++ "'")
        inheritedGlobals
        (commandFlags commandDef)
        (commandArguments commandDef)
        (commandCommands commandDef)
        (commandExclusiveGroups commandDef)
    mapM_ (validateCommand spec globalIds) (commandCommands commandDef)

validateScope :: String -> [FlagDef] -> [FlagDef] -> [ArgumentDef] -> [CommandDef] -> [ExclusiveGroup] -> Either CliBuilderError ()
validateScope scopeName globalFlags localFlags arguments commands groups = do
    rejectDuplicates scopeName "flag id" (map flagId localFlags)
    rejectDuplicates scopeName "argument id" (map argumentId arguments)
    rejectDuplicates scopeName "command id" (map commandId commands)
    mapM_ (validateFlagForm scopeName) localFlags
    let allFlagIds = Set.fromList (map flagId localFlags ++ map flagId globalFlags)
    mapM_ (validateFlagRefs scopeName allFlagIds) localFlags
    mapM_ (validateArgument scopeName allFlagIds) arguments
    validateVariadics scopeName arguments
    validateGroups scopeName allFlagIds groups
    validateRequiresCycles scopeName localFlags

validateFlagsSelfConsistency :: String -> [FlagDef] -> Either CliBuilderError ()
validateFlagsSelfConsistency scopeName flags = do
    rejectDuplicates scopeName "global flag id" (map flagId flags)
    mapM_ (validateFlagForm scopeName) flags
    let ids = Set.fromList (map flagId flags)
    mapM_ (validateFlagRefs scopeName ids) flags
    validateRequiresCycles scopeName flags

validateFlagForm :: String -> FlagDef -> Either CliBuilderError ()
validateFlagForm scopeName flagDef =
    if any isJust [flagShort flagDef, flagLong flagDef, flagSingleDashLong flagDef]
        then Right ()
        else Left (SpecError (scopeName ++ ": flag " ++ show (flagId flagDef) ++ " must define short, long, or single_dash_long"))

validateFlagRefs :: String -> Set String -> FlagDef -> Either CliBuilderError ()
validateFlagRefs scopeName allFlagIds flagDef = do
    mapM_ (requireKnownFlag scopeName allFlagIds (flagId flagDef) "conflicts_with") (flagConflictsWith flagDef)
    mapM_ (requireKnownFlag scopeName allFlagIds (flagId flagDef) "requires") (flagRequires flagDef)
    mapM_ (requireKnownFlag scopeName allFlagIds (flagId flagDef) "required_unless") (flagRequiredUnless flagDef)
    if flagType flagDef == "enum" && null (flagEnumValues flagDef)
        then Left (SpecError (scopeName ++ ": flag " ++ show (flagId flagDef) ++ " has type \"enum\" but enum_values is empty"))
        else Right ()
    case flagDefaultWhenPresent flagDef of
        Nothing -> Right ()
        Just defaultValue ->
            if flagType flagDef /= "enum"
                then Left (SpecError (scopeName ++ ": flag " ++ show (flagId flagDef) ++ " has default_when_present but is not enum"))
                else if defaultValue `elem` flagEnumValues flagDef
                    then Right ()
                    else Left (SpecError (scopeName ++ ": flag " ++ show (flagId flagDef) ++ " default_when_present must be one of enum_values"))

validateArgument :: String -> Set String -> ArgumentDef -> Either CliBuilderError ()
validateArgument scopeName allFlagIds argumentDef = do
    if argumentType argumentDef == "enum" && null (argumentEnumValues argumentDef)
        then Left (SpecError (scopeName ++ ": argument " ++ show (argumentId argumentDef) ++ " has type \"enum\" but enum_values is empty"))
        else Right ()
    mapM_ (\flagRef -> requireKnownFlag scopeName allFlagIds (argumentId argumentDef) "required_unless_flag" flagRef) (argumentRequiredUnlessFlag argumentDef)

validateVariadics :: String -> [ArgumentDef] -> Either CliBuilderError ()
validateVariadics scopeName arguments =
    if length (filter argumentVariadic arguments) <= 1
        then Right ()
        else Left (SpecError (scopeName ++ ": at most one variadic argument is allowed"))

validateGroups :: String -> Set String -> [ExclusiveGroup] -> Either CliBuilderError ()
validateGroups scopeName allFlagIds =
    mapM_
        (\groupValue ->
            mapM_
                (\flagRef ->
                    if Set.member flagRef allFlagIds
                        then Right ()
                        else Left (SpecError (scopeName ++ ": exclusive group " ++ show (groupId groupValue) ++ " references unknown flag id " ++ show flagRef))
                )
                (groupFlagIds groupValue)
        )

validateRequiresCycles :: String -> [FlagDef] -> Either CliBuilderError ()
validateRequiresCycles scopeName flags =
    let localIds = Set.fromList (map flagId flags)
        graphMap =
            Map.fromList
                [ (flagId flagDef, filter (`Set.member` localIds) (flagRequires flagDef))
                | flagDef <- flags
                ]
     in if any (hasCycle graphMap []) (Map.keys graphMap)
            then Left (SpecError (scopeName ++ ": circular requires dependency detected"))
            else Right ()

requireKnownFlag :: String -> Set String -> String -> String -> String -> Either CliBuilderError ()
requireKnownFlag scopeName allFlagIds owner relation referenced =
    if Set.member referenced allFlagIds
        then Right ()
        else Left (SpecError (scopeName ++ ": " ++ show owner ++ " " ++ relation ++ " unknown flag id " ++ show referenced))

rejectDuplicates :: String -> String -> [String] -> Either CliBuilderError ()
rejectDuplicates scopeName label values =
    case firstDuplicate values of
        Nothing -> Right ()
        Just duplicateValue -> Left (SpecError (scopeName ++ ": duplicate " ++ label ++ " " ++ show duplicateValue))

firstDuplicate :: [String] -> Maybe String
firstDuplicate values = go Set.empty values
  where
    go _ [] = Nothing
    go seen (value : rest)
        | Set.member value seen = Just value
        | otherwise = go (Set.insert value seen) rest

hasCycle :: Map String [String] -> [String] -> String -> Bool
hasCycle graphMap stack node
    | node `elem` stack = True
    | otherwise = any (hasCycle graphMap (node : stack)) (Map.findWithDefault [] node graphMap)

formatCommandsSection :: [CommandDef] -> String
formatCommandsSection [] = ""
formatCommandsSection commands =
    unlines ("COMMANDS" : map renderCommand commands)
  where
    width = maximum (8 : map (length . commandName) commands) + 2
    renderCommand commandDef =
        "  "
            ++ commandName commandDef
            ++ replicate (width - length (commandName commandDef)) ' '
            ++ commandDescription commandDef

formatOptionsSection :: String -> [FlagDef] -> String
formatOptionsSection _ [] = ""
formatOptionsSection title flags =
    unlines (title : zipWith renderOption signatures flags)
  where
    signatures = map formatFlagSignature flags
    width = maximum (16 : map length signatures) + 2
    renderOption signature flagDef =
        "  "
            ++ signature
            ++ replicate (width - length signature) ' '
            ++ flagDescription flagDef
            ++ maybe "" (\defaultValue -> " [default: " ++ renderJson defaultValue ++ "]") (flagDefault flagDef)

formatArgumentsSection :: [ArgumentDef] -> String
formatArgumentsSection [] = ""
formatArgumentsSection arguments =
    unlines ("ARGUMENTS" : zipWith renderArgument signatures arguments)
  where
    signatures = map formatArgumentSignature arguments
    width = maximum (8 : map length signatures) + 2
    renderArgument signature argumentDef =
        "  "
            ++ signature
            ++ replicate (width - length signature) ' '
            ++ argumentDescription argumentDef
            ++ if argumentRequired argumentDef then " Required." else " Optional."
            ++ if argumentVariadic argumentDef then " Repeatable." else ""
            ++ maybe "" (\defaultValue -> " [default: " ++ renderJson defaultValue ++ "]") (argumentDefault argumentDef)

formatFlagSignature :: FlagDef -> String
formatFlagSignature flagDef =
    intercalate ", " (filter (not . null) [shortPart, longPart, singleDashLongPart]) ++ valueSuffix
  where
    shortPart = maybe "" (\shortValue -> "-" ++ shortValue) (flagShort flagDef)
    longPart = maybe "" (\longValue -> "--" ++ longValue) (flagLong flagDef)
    singleDashLongPart = maybe "" (\value -> "-" ++ value) (flagSingleDashLong flagDef)
    valueSuffix =
        if flagConsumesValue flagDef
            then " <" ++ fromMaybe "VALUE" (flagValueName flagDef) ++ ">"
            else ""

formatArgumentSignature :: ArgumentDef -> String
formatArgumentSignature argumentDef =
    let base =
            if argumentRequired argumentDef
                then "<" ++ argumentDisplayName argumentDef ++ ">"
                else "[" ++ argumentDisplayName argumentDef ++ "]"
     in if argumentVariadic argumentDef
            then base ++ "..."
            else base

formatArgumentUsage :: ArgumentDef -> String
formatArgumentUsage = formatArgumentSignature

hasAnyOptions :: [FlagDef] -> [FlagDef] -> Bool
hasAnyOptions localFlags globalFlags = not (null localFlags && null globalFlags)

builtinFlagsFor :: CliSpec -> [FlagDef]
builtinFlagsFor spec =
    helpFlag ++ versionFlag
  where
    helpFlag =
        if builtinHelp (cliBuiltinFlags spec)
            then
                [ FlagDef "__builtin_help" (Just "h") (Just "help") Nothing "Show this help message and exit" "boolean" False Nothing Nothing [] [] [] [] False Nothing
                ]
            else []
    versionFlag =
        if builtinVersion (cliBuiltinFlags spec) && isJust (cliVersion spec)
            then
                [ FlagDef "__builtin_version" Nothing (Just "version") Nothing "Show version and exit" "boolean" False Nothing Nothing [] [] [] [] False Nothing
                ]
            else []

findCommandByPath :: CliSpec -> [String] -> Maybe CommandDef
findCommandByPath _ [] = Nothing
findCommandByPath spec names = go (cliCommands spec) names
  where
    go _ [] = Nothing
    go commands [nameValue] = find matchesName commands
      where
        matchesName commandDef =
            commandName commandDef == nameValue || nameValue `elem` commandAliases commandDef
    go commands (nameValue : rest) =
        case find (\commandDef -> commandName commandDef == nameValue || nameValue `elem` commandAliases commandDef) commands of
            Nothing -> Nothing
            Just commandDef -> go (commandCommands commandDef) rest

data RoutingResult = RoutingResult
    { routedPath :: [String]
    , routedCommandIndices :: Set Int
    }

routeCommand :: CliSpec -> String -> [String] -> RoutingResult
routeCommand spec program argv = go [program] Set.empty (cliCommands spec) 0
  where
    go commandPath commandIndices commands index
        | index >= length argv = RoutingResult commandPath commandIndices
        | otherwise =
            let token = argv !! index
                activeFlags = fst3 (activeScope spec commandPath)
             in if token == "--"
                    then RoutingResult commandPath commandIndices
                    else if looksLikeFlag token
                        then go commandPath commandIndices commands (index + routeSkip activeFlags token argv index)
                        else
                            case find (\commandDef -> commandName commandDef == token || token `elem` commandAliases commandDef) commands of
                                Nothing -> RoutingResult commandPath commandIndices
                                Just commandDef ->
                                    go
                                        (commandPath ++ [commandName commandDef])
                                        (Set.insert index commandIndices)
                                        (commandCommands commandDef)
                                        (index + 1)

routeSkip :: [FlagDef] -> String -> [String] -> Int -> Int
routeSkip flags token argv index =
    case classifyToken (map flagInfoFromDef (flags ++ builtinFlags flags)) token of
        [LongFlag nameValue] ->
            if maybe False flagConsumesValue (find (\flagDef -> flagLong flagDef == Just nameValue) flags)
                then if index + 1 < length argv then 2 else 1
                else 1
        [ShortFlag shortValue] ->
            if maybe False flagConsumesValue (find (\flagDef -> flagShort flagDef == Just [shortValue]) flags)
                then if index + 1 < length argv then 2 else 1
                else 1
        [SingleDashLong nameValue] ->
            if maybe False flagConsumesValue (find (\flagDef -> flagSingleDashLong flagDef == Just nameValue) flags)
                then if index + 1 < length argv then 2 else 1
                else 1
        [_] -> 1
        _ -> 1
  where
    builtinFlags activeFlags =
        if any ((== "__builtin_help") . flagId) activeFlags || any ((== "__builtin_version") . flagId) activeFlags
            then []
            else []

looksLikeFlag :: String -> Bool
looksLikeFlag ('-' : _ : _) = True
looksLikeFlag _ = False

activeScope :: CliSpec -> [String] -> ([FlagDef], [ArgumentDef], [ExclusiveGroup])
activeScope spec commandPath =
    case findCommandByPath spec (drop 1 commandPath) of
        Nothing -> (cliFlags spec ++ cliGlobalFlags spec, cliArguments spec, cliMutuallyExclusiveGroups spec)
        Just commandDef ->
            ( commandFlags commandDef ++ if commandInheritGlobalFlags commandDef then cliGlobalFlags spec else []
            , commandArguments commandDef
            , commandExclusiveGroups commandDef
            )

data ScanResult = ScanResult
    { scannedParsedFlags :: Map String JsonValue
    , scannedPositionals :: [String]
    , scannedErrors :: [ParseError]
    , scannedHelpRequested :: Bool
    , scannedVersionRequested :: Bool
    , scannedExplicitFlags :: [String]
    }

scanTokens :: CliSpec -> [FlagDef] -> [FlagInfo] -> [String] -> Set Int -> [String] -> ScanResult
scanTokens spec activeFlags classifierFlags commandPath commandIndices argv =
    finalize (foldl step initialState (zip [0 ..] argv))
  where
    initialState = (Map.empty, [], [], Nothing, False, False, False, [])
    isTraditional = cliParsingMode spec == "traditional"
    knownSubcommands = map commandName (cliCommands spec)
    step stateValue (index, token)
        | Set.member index commandIndices = stateValue
        | otherwise =
            case stateValue of
                (parsedFlags, positionals, errors, Just pendingFlag, endOfFlagsSeen, helpRequested, versionRequested, explicitFlags) ->
                    let (newFlags, newErrors) = storeFlagValue parsedFlags pendingFlag token commandPath
                     in (newFlags, positionals, errors ++ newErrors, Nothing, endOfFlagsSeen, helpRequested, versionRequested, explicitFlags)
                (parsedFlags, positionals, errors, Nothing, True, helpRequested, versionRequested, explicitFlags) ->
                    (parsedFlags, positionals ++ [token], errors, Nothing, True, helpRequested, versionRequested, explicitFlags)
                (parsedFlags, positionals, errors, Nothing, False, helpRequested, versionRequested, explicitFlags) ->
                    let events =
                            if isTraditional && index == 0 && not (looksLikeFlag token)
                                then classifyTraditional classifierFlags token knownSubcommands
                                else classifyToken classifierFlags token
                     in processEvents activeFlags commandPath parsedFlags positionals errors explicitFlags helpRequested versionRequested events
    finalize (parsedFlags, positionals, errors, _, endOfFlagsSeen, helpRequested, versionRequested, explicitFlags) =
        let _ = endOfFlagsSeen
         in ScanResult parsedFlags positionals errors helpRequested versionRequested explicitFlags

processEvents ::
       [FlagDef]
    -> [String]
    -> Map String JsonValue
    -> [String]
    -> [ParseError]
    -> [String]
    -> Bool
    -> Bool
    -> [TokenEvent]
    -> (Map String JsonValue, [String], [ParseError], Maybe FlagDef, Bool, Bool, Bool, [String])
processEvents activeFlags commandPath parsedFlags positionals errors explicitFlags helpRequested versionRequested =
    foldl step (parsedFlags, positionals, errors, Nothing, False, helpRequested, versionRequested, explicitFlags)
  where
    step stateValue event =
        case stateValue of
            (flagsMap, currentPositionals, currentErrors, pendingFlag, endOfFlagsSeen, helpValue, versionValue, explicitValue) ->
                case event of
                    EndOfFlags ->
                        (flagsMap, currentPositionals, currentErrors, pendingFlag, True, helpValue, versionValue, explicitValue)
                    Positional tokenValue ->
                        (flagsMap, currentPositionals ++ [tokenValue], currentErrors, pendingFlag, endOfFlagsSeen, helpValue, versionValue, explicitValue)
                    UnknownFlag rawFlag ->
                        let suggestion = suggestFlag rawFlag activeFlags
                            nextError = ParseError "unknown_flag" ("Unknown flag " ++ rawFlag) suggestion commandPath
                         in (flagsMap, currentPositionals, currentErrors ++ [nextError], pendingFlag, endOfFlagsSeen, helpValue, versionValue, explicitValue)
                    LongFlag nameValue ->
                        consumeFlagBy
                            (\flagDef -> flagLong flagDef == Just nameValue || (flagId flagDef == "__builtin_help" && nameValue == "help") || (flagId flagDef == "__builtin_version" && nameValue == "version"))
                            nameValue
                            Nothing
                            stateValue
                    LongFlagWithValue nameValue inlineValue ->
                        consumeFlagBy
                            (\flagDef -> flagLong flagDef == Just nameValue)
                            nameValue
                            (Just inlineValue)
                            stateValue
                    SingleDashLong nameValue ->
                        consumeFlagBy
                            (\flagDef -> flagSingleDashLong flagDef == Just nameValue)
                            nameValue
                            Nothing
                            stateValue
                    ShortFlag shortValue ->
                        consumeFlagBy
                            (\flagDef -> flagShort flagDef == Just [shortValue] || (flagId flagDef == "__builtin_help" && shortValue == 'h'))
                            [shortValue]
                            Nothing
                            stateValue
                    ShortFlagWithValue shortValue inlineValue ->
                        consumeFlagBy
                            (\flagDef -> flagShort flagDef == Just [shortValue])
                            [shortValue]
                            (Just inlineValue)
                            stateValue
                    StackedFlags chars ->
                        foldl
                            (\innerStateValue shortValue ->
                                consumeFlagBy
                                    (\flagDef -> flagShort flagDef == Just [shortValue] || (flagId flagDef == "__builtin_help" && shortValue == 'h'))
                                    [shortValue]
                                    Nothing
                                    innerStateValue
                            )
                            stateValue
                            chars
    consumeFlagBy matcher rawValue inlineMaybe stateValue =
        case stateValue of
            (flagsMap, currentPositionals, currentErrors, pendingFlag, endOfFlagsSeen, helpValue, versionValue, explicitValue) ->
                case find matcher activeFlags of
                    Nothing ->
                        let nextError = ParseError "unknown_flag" ("Unknown flag " ++ rawValue) (suggestFlag rawValue activeFlags) commandPath
                         in (flagsMap, currentPositionals, currentErrors ++ [nextError], pendingFlag, endOfFlagsSeen, helpValue, versionValue, explicitValue)
                    Just flagDef
                        | flagId flagDef == "__builtin_help" ->
                            (flagsMap, currentPositionals, currentErrors, pendingFlag, endOfFlagsSeen, True, versionValue, explicitValue)
                        | flagId flagDef == "__builtin_version" ->
                            (flagsMap, currentPositionals, currentErrors, pendingFlag, endOfFlagsSeen, helpValue, True, explicitValue)
                        | flagType flagDef == "boolean" ->
                            (storeDirect flagsMap flagDef (JsonBool True), currentPositionals, currentErrors, pendingFlag, endOfFlagsSeen, helpValue, versionValue, explicitValue ++ [flagId flagDef])
                        | flagType flagDef == "count" ->
                            (incrementCount flagsMap flagDef, currentPositionals, currentErrors, pendingFlag, endOfFlagsSeen, helpValue, versionValue, explicitValue ++ [flagId flagDef])
                        | isJust (flagDefaultWhenPresent flagDef) && inlineMaybe == Nothing ->
                            let valueText = fromMaybe "" (flagDefaultWhenPresent flagDef)
                             in (storeDirect flagsMap flagDef (JsonString valueText), currentPositionals, currentErrors, pendingFlag, endOfFlagsSeen, helpValue, versionValue, explicitValue ++ [flagId flagDef])
                        | otherwise ->
                            case inlineMaybe of
                                Nothing ->
                                    (flagsMap, currentPositionals, currentErrors, Just flagDef, endOfFlagsSeen, helpValue, versionValue, explicitValue ++ [flagId flagDef])
                                Just inlineValue ->
                                    let (newFlags, newErrors) = storeFlagValue flagsMap flagDef inlineValue commandPath
                                     in (newFlags, currentPositionals, currentErrors ++ newErrors, pendingFlag, endOfFlagsSeen, helpValue, versionValue, explicitValue ++ [flagId flagDef])

storeFlagValue :: Map String JsonValue -> FlagDef -> String -> [String] -> (Map String JsonValue, [ParseError])
storeFlagValue flagsMap flagDef tokenValue commandPath =
    case coerceValue (flagType flagDef) (flagEnumValues flagDef) tokenValue of
        Left errorMessage ->
            ( flagsMap
            , [ParseError "invalid_flag_value" ("Flag " ++ show (flagId flagDef) ++ " " ++ errorMessage) Nothing commandPath]
            )
        Right jsonValue ->
            (storeDirect flagsMap flagDef jsonValue, [])

storeDirect :: Map String JsonValue -> FlagDef -> JsonValue -> Map String JsonValue
storeDirect flagsMap flagDef jsonValue =
    if flagRepeatable flagDef
        then Map.insertWith appendJsonArray (flagId flagDef) (JsonArray [jsonValue]) flagsMap
        else Map.insert (flagId flagDef) jsonValue flagsMap
  where
    appendJsonArray (JsonArray leftValues) (JsonArray rightValues) = JsonArray (rightValues ++ leftValues)
    appendJsonArray newValue _ = newValue

incrementCount :: Map String JsonValue -> FlagDef -> Map String JsonValue
incrementCount flagsMap flagDef =
    let current =
            case Map.lookup (flagId flagDef) flagsMap of
                Just (JsonNumber value) -> value
                _ -> 0
     in Map.insert (flagId flagDef) (JsonNumber (current + 1)) flagsMap

coerceValue :: String -> [String] -> String -> Either String JsonValue
coerceValue typeName enumValues tokenValue =
    case typeName of
        "string" -> Right (JsonString tokenValue)
        "path" -> Right (JsonString tokenValue)
        "file" -> Right (JsonString tokenValue)
        "directory" -> Right (JsonString tokenValue)
        "integer" ->
            case reads tokenValue of
                [(value, "")] -> Right (JsonNumber value)
                _ -> Left "expected integer"
        "float" ->
            case reads tokenValue of
                [(value, "")] -> Right (JsonNumber value)
                _ -> Left "expected float"
        "boolean" ->
            case tokenValue of
                "true" -> Right (JsonBool True)
                "false" -> Right (JsonBool False)
                _ -> Left "expected boolean"
        "enum" ->
            if tokenValue `elem` enumValues
                then Right (JsonString tokenValue)
                else Left ("must be one of " ++ show enumValues)
        _ -> Right (JsonString tokenValue)

resolveArguments :: [String] -> [ArgumentDef] -> [String] -> Map String JsonValue -> ([ParseError], Map String JsonValue)
resolveArguments commandPath arguments tokens parsedFlags = go arguments tokens Map.empty []
  where
    go [] [] assignments errors = (reverse errors, assignments)
    go [] remaining assignments errors =
        let extraErrors =
                [ ParseError "unexpected_argument" ("Unexpected positional argument " ++ show value) Nothing commandPath
                | value <- remaining
                ]
         in (reverse (extraErrors ++ errors), assignments)
    go (argumentDef : rest) remaining assignments errors
        | argumentVariadic argumentDef =
            let reserve = minimumTokensFor rest parsedFlags
                available = length remaining - reserve
                minCount = max 0 (argumentVariadicMin argumentDef)
                desiredCount =
                    max minCount
                        (case argumentVariadicMax argumentDef of
                            Nothing -> max minCount available
                            Just maxCount -> min maxCount (max 0 available)
                        )
             in if available < minCount
                    then
                        let nextErrors =
                                if argumentRequired argumentDef
                                    then ParseError "missing_required_argument" ("Missing required argument " ++ show (argumentDisplayName argumentDef)) Nothing commandPath : errors
                                    else errors
                            assignment =
                                case argumentDefault argumentDef of
                                    Just defaultValue -> defaultValue
                                    Nothing -> JsonArray []
                         in go rest remaining (Map.insert (argumentId argumentDef) assignment assignments) nextErrors
                    else
                        let (consumed, restTokens) = splitAt desiredCount remaining
                            assignment =
                                if null consumed
                                    then fromMaybe (JsonArray []) (argumentDefault argumentDef)
                                    else JsonArray (map JsonString consumed)
                         in go rest restTokens (Map.insert (argumentId argumentDef) assignment assignments) errors
        | otherwise =
            case remaining of
                [] ->
                    let nextErrors =
                            if argumentRequired argumentDef && not (argumentOptionalBecauseFlag argumentDef parsedFlags)
                                then ParseError "missing_required_argument" ("Missing required argument " ++ show (argumentDisplayName argumentDef)) Nothing commandPath : errors
                                else errors
                        assignment = fromMaybe JsonNull (argumentDefault argumentDef)
                     in go rest [] (Map.insert (argumentId argumentDef) assignment assignments) nextErrors
                tokenValue : restTokens ->
                    case coerceValue (argumentType argumentDef) (argumentEnumValues argumentDef) tokenValue of
                        Left err ->
                            go
                                rest
                                restTokens
                                (Map.insert (argumentId argumentDef) JsonNull assignments)
                                (ParseError "invalid_argument_value" ("Argument " ++ show (argumentDisplayName argumentDef) ++ " " ++ err) Nothing commandPath : errors)
                        Right coercedValue ->
                            go rest restTokens (Map.insert (argumentId argumentDef) coercedValue assignments) errors

minimumTokensFor :: [ArgumentDef] -> Map String JsonValue -> Int
minimumTokensFor arguments parsedFlags =
    sum
        [ if argumentVariadic argumentDef
            then max 0 (argumentVariadicMin argumentDef)
            else if argumentRequired argumentDef && not (argumentOptionalBecauseFlag argumentDef parsedFlags)
                then 1
                else 0
        | argumentDef <- arguments
        ]

argumentOptionalBecauseFlag :: ArgumentDef -> Map String JsonValue -> Bool
argumentOptionalBecauseFlag argumentDef parsedFlags =
    any (`flagPresent` parsedFlags) (argumentRequiredUnlessFlag argumentDef)

validateParsedFlags :: [String] -> [FlagDef] -> [ExclusiveGroup] -> [String] -> [ParseError]
validateParsedFlags commandPath activeFlags groups explicitFlags =
    concatMap validateFlag activeFlags ++ concatMap validateGroup groups
  where
    presentIds = Set.fromList explicitFlags
    validateFlag flagDef =
        requiredErrors ++ conflictErrors ++ requiresErrors ++ unlessErrors
      where
        present = Set.member (flagId flagDef) presentIds
        requiredErrors =
            if flagRequired flagDef && not present
                then [ParseError "missing_required_flag" ("Missing required flag " ++ show (flagId flagDef)) Nothing commandPath]
                else []
        conflictErrors =
            [ ParseError "conflicting_flags" ("Flag " ++ show (flagId flagDef) ++ " conflicts with " ++ show otherFlag) Nothing commandPath
            | present
            , otherFlag <- flagConflictsWith flagDef
            , Set.member otherFlag presentIds
            ]
        requiresErrors =
            [ ParseError "missing_dependency_flag" ("Flag " ++ show (flagId flagDef) ++ " requires " ++ show otherFlag) Nothing commandPath
            | present
            , otherFlag <- flagRequires flagDef
            , not (Set.member otherFlag presentIds)
            ]
        unlessErrors =
            if present || null (flagRequiredUnless flagDef) || any (`Set.member` presentIds) (flagRequiredUnless flagDef)
                then []
                else [ParseError "missing_required_flag" ("Flag " ++ show (flagId flagDef) ++ " is required unless one of " ++ show (flagRequiredUnless flagDef) ++ " is present") Nothing commandPath]
    validateGroup groupValue =
        let countPresent = length (filter (`Set.member` presentIds) (groupFlagIds groupValue))
         in [ ParseError "exclusive_group_violation" ("Exclusive group " ++ show (groupId groupValue) ++ " allows at most one of " ++ show (groupFlagIds groupValue)) Nothing commandPath
            | countPresent > 1
            ]
                ++ [ ParseError "missing_required_flag" ("Exclusive group " ++ show (groupId groupValue) ++ " requires exactly one member") Nothing commandPath
                   | groupRequired groupValue && countPresent /= 1
                   ]

populateFlagDefaults :: [FlagDef] -> Map String JsonValue -> Map String JsonValue
populateFlagDefaults activeFlags parsedFlags =
    foldl applyDefault parsedFlags activeFlags
  where
    applyDefault acc flagDef
        | Map.member (flagId flagDef) acc = acc
        | flagType flagDef == "boolean" = Map.insert (flagId flagDef) (JsonBool False) acc
        | flagType flagDef == "count" = Map.insert (flagId flagDef) (JsonNumber 0) acc
        | flagRepeatable flagDef = Map.insert (flagId flagDef) (JsonArray []) acc
        | otherwise = Map.insert (flagId flagDef) (fromMaybe JsonNull (flagDefault flagDef)) acc

flagPresent :: String -> Map String JsonValue -> Bool
flagPresent flagName parsedFlags =
    case Map.lookup flagName parsedFlags of
        Just (JsonBool True) -> True
        Just (JsonNumber numberValue) -> numberValue /= 0
        Just JsonNull -> False
        Just (JsonArray values) -> not (null values)
        Just _ -> True
        Nothing -> False

suggestFlag :: String -> [FlagDef] -> Maybe String
suggestFlag rawFlag flags =
    fmap formatCandidate
        (find (\candidate -> editDistance (normalize rawFlag) candidate <= 2) candidates)
  where
    candidates =
        nub
            (mapMaybe flagLong flags
                ++ mapMaybe flagSingleDashLong flags
                ++ mapMaybe flagShort flags
            )
    normalize value =
        case dropWhile (== '-') value of
            [] -> value
            stripped -> stripped
    formatCandidate candidate =
        if length candidate == 1
            then "-" ++ candidate
            else "--" ++ candidate

editDistance :: String -> String -> Int
editDistance left right = last (foldl transform [0 .. length right] (zip [1 ..] left))
  where
    transform previousRow (leftIndex, leftChar) =
        scanl compute leftIndex (zip3 right [1 ..] previousRow)
      where
        compute leftCost (rightChar, rightIndex, diagonal) =
            minimum
                [ previousRow !! rightIndex + 1
                , leftCost + 1
                , diagonal + if leftChar == rightChar then 0 else 1
                ]

flagInfoFromDef :: FlagDef -> FlagInfo
flagInfoFromDef flagDef =
    FlagInfo
        (flagId flagDef)
        (flagShort flagDef >>= firstChar)
        (flagLong flagDef)
        (flagSingleDashLong flagDef)
        (flagType flagDef == "boolean" || flagType flagDef == "count" || isJust (flagDefaultWhenPresent flagDef))
        (flagType flagDef == "count")
        (isJust (flagDefaultWhenPresent flagDef))

flagConsumesValue :: FlagDef -> Bool
flagConsumesValue flagDef =
    not
        (flagType flagDef == "boolean"
            || flagType flagDef == "count"
            || isJust (flagDefaultWhenPresent flagDef)
        )

classifyLongToken :: [FlagInfo] -> String -> [TokenEvent]
classifyLongToken flagInfos rest =
    case break (== '=') rest of
        (nameValue, '=' : valuePart) ->
            if isKnownLong nameValue flagInfos
                then [LongFlagWithValue nameValue valuePart]
                else [UnknownFlag ("--" ++ nameValue)]
        _ ->
            if isKnownLong rest flagInfos || rest == "help" || rest == "version"
                then [LongFlag rest]
                else [UnknownFlag ("--" ++ rest)]

classifyShortOrSingleDashLong :: [FlagInfo] -> String -> String -> [TokenEvent]
classifyShortOrSingleDashLong flagInfos rest originalToken =
    case find (\flagInfo -> infoSingleDashLong flagInfo == Just rest) flagInfos of
        Just _ -> [SingleDashLong rest]
        Nothing ->
            case rest of
                [singleChar] ->
                    if isKnownShort singleChar flagInfos
                        then [ShortFlag singleChar]
                        else [UnknownFlag originalToken]
                firstChar : remainder ->
                    case find (\flagInfo -> infoShort flagInfo == Just firstChar) flagInfos of
                        Nothing -> [UnknownFlag originalToken]
                        Just flagInfo ->
                            if not (infoBoolean flagInfo)
                                then [ShortFlagWithValue firstChar remainder]
                                else decomposeStack flagInfos [firstChar] remainder originalToken
                [] -> [UnknownFlag originalToken]

decomposeStack :: [FlagInfo] -> [Char] -> String -> String -> [TokenEvent]
decomposeStack flagInfos seen [] _ =
    case seen of
        [singleChar] -> [ShortFlag singleChar]
        _ -> [StackedFlags seen]
decomposeStack flagInfos seen (nextChar : rest) originalToken =
    case find (\flagInfo -> infoShort flagInfo == Just nextChar) flagInfos of
        Nothing -> [UnknownFlag originalToken]
        Just flagInfo ->
            if infoBoolean flagInfo
                then decomposeStack flagInfos (seen ++ [nextChar]) rest originalToken
                else
                    let prefixEvents =
                            if null seen
                                then []
                                else
                                    case seen of
                                        [singleChar] -> [ShortFlag singleChar]
                                        _ -> [StackedFlags seen]
                        finalEvent =
                            if null rest
                                then ShortFlag nextChar
                                else ShortFlagWithValue nextChar rest
                     in prefixEvents ++ [finalEvent]

isKnownLong :: String -> [FlagInfo] -> Bool
isKnownLong nameValue = any (\flagInfo -> infoLong flagInfo == Just nameValue)

isKnownShort :: Char -> [FlagInfo] -> Bool
isKnownShort shortValue = any (\flagInfo -> infoShort flagInfo == Just shortValue)

requireString :: String -> [(String, JsonValue)] -> Either CliBuilderError String
requireString fieldName objectValue =
    case lookupField fieldName objectValue of
        Nothing -> Left (JsonError ("missing field " ++ show fieldName))
        Just value -> requireJsonString value

optionalString :: String -> [(String, JsonValue)] -> Either CliBuilderError (Maybe String)
optionalString fieldName objectValue =
    case lookupField fieldName objectValue of
        Nothing -> Right Nothing
        Just value -> Just <$> requireJsonString value

optionalBool :: String -> [(String, JsonValue)] -> Either CliBuilderError (Maybe Bool)
optionalBool fieldName objectValue =
    case lookupField fieldName objectValue of
        Nothing -> Right Nothing
        Just (JsonBool boolValue) -> Right (Just boolValue)
        Just _ -> Left (JsonError ("field " ++ show fieldName ++ " must be boolean"))

optionalInt :: String -> [(String, JsonValue)] -> Either CliBuilderError (Maybe Int)
optionalInt fieldName objectValue =
    case lookupField fieldName objectValue of
        Nothing -> Right Nothing
        Just (JsonNumber numberValue) -> Right (Just (round numberValue))
        Just _ -> Left (JsonError ("field " ++ show fieldName ++ " must be a number"))

optionalJson :: String -> [(String, JsonValue)] -> Either CliBuilderError (Maybe JsonValue)
optionalJson fieldName objectValue = Right (lookupField fieldName objectValue)

arrayFieldWithDefault :: String -> (JsonValue -> Either CliBuilderError a) -> [(String, JsonValue)] -> Either CliBuilderError [a]
arrayFieldWithDefault fieldName decoder objectValue =
    case lookupField fieldName objectValue of
        Nothing -> Right []
        Just value -> arrayFieldValue fieldName decoder value

arrayField :: String -> (JsonValue -> Either CliBuilderError a) -> [(String, JsonValue)] -> Either CliBuilderError [a]
arrayField fieldName decoder objectValue =
    case lookupField fieldName objectValue of
        Nothing -> Left (JsonError ("missing field " ++ show fieldName))
        Just value -> arrayFieldValue fieldName decoder value

stringArrayField :: String -> [(String, JsonValue)] -> Either CliBuilderError [String]
stringArrayField fieldName objectValue =
    arrayFieldWithDefault fieldName requireJsonString objectValue

arrayFieldValue :: String -> (JsonValue -> Either CliBuilderError a) -> JsonValue -> Either CliBuilderError [a]
arrayFieldValue fieldName decoder value =
    case value of
        JsonArray values -> mapM decoder values
        _ -> Left (JsonError ("field " ++ show fieldName ++ " must be an array"))

requireJsonString :: JsonValue -> Either CliBuilderError String
requireJsonString value =
    case value of
        JsonString textValue -> Right textValue
        _ -> Left (JsonError "expected string")

asObject :: String -> JsonValue -> Either CliBuilderError [(String, JsonValue)]
asObject label value =
    case value of
        JsonObject fields -> Right fields
        _ -> Left (JsonError (label ++ " must be an object"))

lookupField :: String -> [(String, JsonValue)] -> Maybe JsonValue
lookupField fieldName = lookup fieldName

eitherToMaybe :: Either a b -> Maybe b
eitherToMaybe value =
    case value of
        Left _ -> Nothing
        Right inner -> Just inner

defaultBuiltins :: BuiltinFlags
defaultBuiltins = BuiltinFlags True True

stripPrefix :: String -> String -> Maybe String
stripPrefix prefixValue textValue =
    if prefixValue == take (length prefixValue) textValue
        then Just (drop (length prefixValue) textValue)
        else Nothing

firstChar :: String -> Maybe Char
firstChar [] = Nothing
firstChar (charValue : _) = Just charValue

fst3 :: (a, b, c) -> a
fst3 (value, _, _) = value

showCliBuilderError :: CliBuilderError -> String
showCliBuilderError err =
    case err of
        SpecError messageValue -> "spec error: " ++ messageValue
        JsonError messageValue -> "json error: " ++ messageValue
        IoError messageValue -> "io error: " ++ messageValue
        ParseFailure (ParseErrors errs) ->
            intercalate "\n" [parseErrorType parseError ++ ": " ++ parseErrorMessage parseError | parseError <- errs]
