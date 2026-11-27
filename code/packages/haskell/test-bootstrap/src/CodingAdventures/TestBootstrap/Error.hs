-- | The engine's own exception type.
--
-- Three things go wrong in a property engine for reasons that are the
-- caller's fault or the data's fault, and each one needs a distinct,
-- type-level name because the runner classifies exceptions by TYPE ONLY
-- (never by reading their message text):
--
-- * 'ArgumentError'   -- a public entry point was called with arguments it
--   refuses (min > max, a negative step budget, ...).  Fail closed: it is
--   never a pass.
-- * 'NotFalsifying'   -- a shrinker was asked to start from a value that does
--   NOT falsify the property.  A caller bug, loud by design.
-- * 'ExhaustedError'  -- generation could not produce a value (a filter that
--   never accepts, a spent budget).  The runner reports it as
--   @property-exhausted@.
--
-- None of these is an asynchronous exception, so they are ordinary
-- recoverable failures (see "CodingAdventures.TestBootstrap.Pbt.Safe").
module CodingAdventures.TestBootstrap.Error
  ( PbtError (..)
  , argError
  ) where

import Control.Exception (Exception (..), throwIO)

data PbtError
  = ArgumentError String
  | NotFalsifying
  | ExhaustedError String
  deriving (Eq, Show)

instance Exception PbtError where
  displayException (ArgumentError m) = m
  displayException NotFalsifying = "the initial value must falsify the property"
  displayException (ExhaustedError m) = m

-- | Throws an 'ArgumentError' in 'IO'.
argError :: String -> IO a
argError = throwIO . ArgumentError
