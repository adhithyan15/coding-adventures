-- | Exception classification (VER05n 4.2).
--
-- @SomeException@ also catches @ThreadKilled@, @UserInterrupt@,
-- @StackOverflow@, @HeapOverflow@ and @System.Timeout@'s exception.  An engine
-- that swallowed them would defeat @timeout@, @killThread@ and Ctrl-C, and
-- would report a verdict for a property that overflowed the stack.  So:
--
-- * 'tryRecoverable' RETHROWS every asynchronous exception (anything wrapped
--   in 'SomeAsyncException', the base class of all of them) and turns every
--   other exception into a 'Left'.
-- * Classification is by exception TYPE only.  The text of @displayException@
--   is never read to decide anything: @ErrorCall "ThreadKilled"@ is an
--   ordinary panic.
module CodingAdventures.TestBootstrap.Pbt.Safe
  ( isAsync
  , tryRecoverable
  , exceptionTypeName
  , safeMessage
  ) where

import Control.DeepSeq (force)
import Control.Exception
import Data.Text (Text)
import qualified Data.Text as T
import Data.Typeable (typeOf)

import CodingAdventures.TestBootstrap.CorpusJson.Escape (clipText, escapeText)

-- | True for every asynchronous exception.
isAsync :: SomeException -> Bool
isAsync e = case fromException e of
  Just (SomeAsyncException _) -> True
  Nothing -> False

-- | Runs the action; a synchronous exception becomes 'Left', an asynchronous
-- one is rethrown.  The action is responsible for forcing whatever it returns
-- INSIDE the action (use 'evaluate'): a lazy value returned here escapes the
-- handler.
tryRecoverable :: IO a -> IO (Either SomeException a)
tryRecoverable act = do
  r <- try act
  case r of
    Left e | isAsync e -> throwIO e
    _ -> pure r

-- | The exception's type name (never its message), escaped.
exceptionTypeName :: SomeException -> Text
exceptionTypeName (SomeException e) = escapeText False (T.pack (show (typeOf e)))

-- | A bounded, escaped rendering of an exception's message for NON-redacted
-- output only.  Forced inside a handler (a message can itself be bottom),
-- clipped to 200 characters and escaped.
safeMessage :: SomeException -> IO Text
safeMessage e = do
  r <- tryRecoverable (evaluate (force (take 200 (displayException e))))
  pure $ case r of
    Right s -> escapeText False (clipText 200 (T.pack s))
    Left _ -> "<unprintable>"
