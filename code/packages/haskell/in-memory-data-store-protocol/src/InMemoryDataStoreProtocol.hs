module InMemoryDataStoreProtocol
    ( description
    , CommandFrame(..)
    , commandFrameFromParts
    , commandFrameFromResp
    , EngineResponse(..)
    , engineResponseToResp
    ) where

import qualified Data.ByteString as BS
import qualified Data.ByteString.Char8 as BC
import Data.Char (toUpper)
import RespProtocol
    ( RespValue(..)
    )

description :: String
description = "Haskell in-memory data store protocol intermediate representation"

data CommandFrame = CommandFrame
    { commandFrameCommand :: String
    , commandFrameArgs :: [BS.ByteString]
    }
    deriving (Eq, Show)

commandFrameFromParts :: [BS.ByteString] -> Maybe CommandFrame
commandFrameFromParts parts =
    case parts of
        [] -> Nothing
        commandPart : argParts ->
            Just
                CommandFrame
                    { commandFrameCommand = map toUpper (BC.unpack commandPart)
                    , commandFrameArgs = argParts
                    }

commandFrameFromResp :: RespValue -> Maybe CommandFrame
commandFrameFromResp respValue =
    case respValue of
        RespArray (Just parts) ->
            commandFrameFromParts =<< traverse extractBulkString parts
        _ -> Nothing
  where
    extractBulkString value =
        case value of
            RespBulkString (Just bytesValue) -> Just bytesValue
            _ -> Nothing

data EngineResponse
    = EngineSimpleString String
    | EngineError String
    | EngineInteger Integer
    | EngineBulkString (Maybe BS.ByteString)
    | EngineArray (Maybe [EngineResponse])
    deriving (Eq, Show)

engineResponseToResp :: EngineResponse -> RespValue
engineResponseToResp response =
    case response of
        EngineSimpleString textValue -> RespSimpleString textValue
        EngineError textValue -> RespError textValue
        EngineInteger numberValue -> RespInteger numberValue
        EngineBulkString bytesValue -> RespBulkString bytesValue
        EngineArray valuesValue -> RespArray (fmap (map engineResponseToResp) valuesValue)
