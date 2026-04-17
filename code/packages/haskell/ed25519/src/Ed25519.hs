module Ed25519
    ( description
    , generateKeypair
    , sign
    , verify
    ) where

import Control.Exception (IOException, bracket, catch)
import qualified Data.ByteString as BS
import Data.Word (Word8)
import System.Directory (findExecutable, getTemporaryDirectory, removeFile)
import System.Exit (ExitCode(..))
import System.IO (hClose, openBinaryTempFile)
import System.IO.Error (catchIOError)
import System.Process (readProcessWithExitCode)

description :: String
description = "Ed25519 digital signatures (RFC 8032) backed by the local OpenSSL toolchain"

generateKeypair :: [Word8] -> IO (Either String ([Word8], [Word8]))
generateKeypair seed
    | length seed /= 32 =
        pure (Left ("Ed25519 seed must be 32 bytes, got " ++ show (length seed)))
    | otherwise =
        withTempBinaryFile "ed25519-private-" (ed25519PrivatePrefix <> seed) $ \privatePath ->
            withEmptyTempFile "ed25519-public-" $ \publicPath -> do
                runResult <-
                    runOpenSSL
                        [ "pkey"
                        , "-inform", "DER"
                        , "-in", privatePath
                        , "-outform", "DER"
                        , "-pubout"
                        , "-out", publicPath
                        ]
                case runResult of
                    Left err -> pure (Left err)
                    Right () -> do
                        publicDer <- BS.unpack <$> BS.readFile publicPath
                        pure $
                            case extractPrefixed ed25519PublicPrefix publicDer "Ed25519 public key" of
                                Left err -> Left err
                                Right publicKey -> Right (publicKey, seed <> publicKey)

sign :: [Word8] -> [Word8] -> IO (Either String [Word8])
sign message secretKey
    | length secretKey /= 64 =
        pure (Left ("Ed25519 secret key must be 64 bytes, got " ++ show (length secretKey)))
    | otherwise = do
        generated <- generateKeypair (take 32 secretKey)
        case generated of
            Left err -> pure (Left err)
            Right (_, reconstructedSecret) ->
                if reconstructedSecret /= secretKey
                    then pure (Left "Ed25519 secret key must be seed || public_key")
                    else
                        withTempBinaryFile "ed25519-private-" (ed25519PrivatePrefix <> take 32 secretKey) $ \privatePath ->
                            withTempBinaryFile "ed25519-message-" message $ \messagePath ->
                                withEmptyTempFile "ed25519-signature-" $ \signaturePath -> do
                                    runResult <-
                                        runOpenSSL
                                            [ "dgst"
                                            , "-sign", privatePath
                                            , "-keyform", "DER"
                                            , "-binary"
                                            , "-out", signaturePath
                                            , messagePath
                                            ]
                                    case runResult of
                                        Left err -> pure (Left err)
                                        Right () -> do
                                            signature <- BS.unpack <$> BS.readFile signaturePath
                                            pure $
                                                if length signature == 64
                                                    then Right signature
                                                    else Left ("OpenSSL returned " ++ show (length signature) ++ " bytes for Ed25519 signature")

verify :: [Word8] -> [Word8] -> [Word8] -> IO Bool
verify message signature publicKey
    | length signature /= 64 = pure False
    | length publicKey /= 32 = pure False
    | otherwise =
        withTempBinaryFile "ed25519-public-" (ed25519PublicPrefix <> publicKey) $ \publicPath ->
            withTempBinaryFile "ed25519-message-" message $ \messagePath ->
                withTempBinaryFile "ed25519-signature-" signature $ \signaturePath -> do
                    runResult <-
                        runOpenSSL
                            [ "dgst"
                            , "-verify", publicPath
                            , "-keyform", "DER"
                            , "-signature", signaturePath
                            , messagePath
                            ]
                    pure $
                        case runResult of
                            Right () -> True
                            Left _ -> False

ed25519PrivatePrefix :: [Word8]
ed25519PrivatePrefix = hexToBytes "302e020100300506032b657004220420"

ed25519PublicPrefix :: [Word8]
ed25519PublicPrefix = hexToBytes "302a300506032b6570032100"

extractPrefixed :: [Word8] -> [Word8] -> String -> Either String [Word8]
extractPrefixed prefixValue encoded label
    | take (length prefixValue) encoded /= prefixValue =
        Left ("Unexpected OpenSSL DER output for " ++ label)
    | otherwise =
        Right (drop (length prefixValue) encoded)

runOpenSSL :: [String] -> IO (Either String ())
runOpenSSL arguments = do
    maybePath <- findExecutable "openssl"
    case maybePath of
        Nothing -> pure (Left "openssl executable was not found on PATH")
        Just executablePath -> do
            result <-
                catch
                    (do
                        (exitCode, _, stderrOutput) <- readProcessWithExitCode executablePath arguments ""
                        pure (Right (exitCode, stderrOutput)))
                    (\err -> pure (Left (show (err :: IOException))))
            pure $
                case result of
                    Left err -> Left err
                    Right (ExitSuccess, _) -> Right ()
                    Right (ExitFailure _, stderrOutput) ->
                        Left ("OpenSSL failed: " ++ trim stderrOutput)

withTempBinaryFile :: String -> [Word8] -> (FilePath -> IO a) -> IO a
withTempBinaryFile template contents =
    bracket create cleanup
  where
    create = do
        tempDirectory <- getTemporaryDirectory
        (path, handle) <- openBinaryTempFile tempDirectory template
        BS.hPut handle (BS.pack contents)
        hClose handle
        pure path
    cleanup path = ignoreIo (removeFile path)

withEmptyTempFile :: String -> (FilePath -> IO a) -> IO a
withEmptyTempFile template =
    bracket create cleanup
  where
    create = do
        tempDirectory <- getTemporaryDirectory
        (path, handle) <- openBinaryTempFile tempDirectory template
        hClose handle
        pure path
    cleanup path = ignoreIo (removeFile path)

ignoreIo :: IO () -> IO ()
ignoreIo action =
    catchIOError action (\_ -> pure ())

trim :: String -> String
trim =
    reverse . dropWhile (`elem` ['\n', '\r', ' ']) . reverse

hexToBytes :: String -> [Word8]
hexToBytes [] = []
hexToBytes (a : b : rest) = fromIntegral (hexDigit a * 16 + hexDigit b) : hexToBytes rest
hexToBytes _ = []

hexDigit :: Char -> Int
hexDigit character
    | character >= '0' && character <= '9' = fromEnum character - fromEnum '0'
    | character >= 'a' && character <= 'f' = 10 + fromEnum character - fromEnum 'a'
    | character >= 'A' && character <= 'F' = 10 + fromEnum character - fromEnum 'A'
    | otherwise = 0
