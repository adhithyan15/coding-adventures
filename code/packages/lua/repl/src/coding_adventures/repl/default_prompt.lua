-- default_prompt — Standard two-level prompt strings for the REPL
--
-- # What Is a Prompt Plug-in?
--
-- A Prompt object controls what the user sees on screen before typing. The
-- REPL framework calls two methods at different points in the input cycle:
--
--   prompt.global_prompt() → string
--     Shown at the top level, when the REPL is waiting for a new expression.
--     Typically something short like "> " or "lua> ".
--
--   prompt.line_prompt() → string
--     Shown on continuation lines, when the user has started a multi-line
--     expression and the REPL is waiting for more. Typically "... ".
--
-- Both return strings (not print them). The REPL loop decides when and how to
-- display them, keeping the Prompt object free from I/O concerns. This
-- separation makes it easy to swap in a coloured prompt, a prompt that shows
-- the current file name, or a no-op prompt for batch mode.
--
-- # Why Two Prompts?
--
-- Think of how Python's interactive interpreter works:
--
--   >>> def foo():    ← global_prompt: we are at the top level
--   ...   return 1   ← line_prompt: we are inside a definition
--   >>>
--
-- The visual distinction helps users know whether pressing Enter will submit
-- the expression or continue building it.
--
-- # DefaultPrompt
--
-- The default implementation uses the traditional Unix conventions:
--
--   global_prompt → "> "
--   line_prompt   → "... "
--
-- These match the prompts used by Lua's own interactive interpreter (`lua -i`),
-- so they feel immediately familiar.

local DefaultPrompt = {}

-- global_prompt() — the primary prompt shown before each new expression.
--
-- Returns:
--   string — e.g. "> "
--
-- Note the trailing space: this puts a gap between the prompt character and
-- the user's cursor, which is easier to read than ">cursor".
function DefaultPrompt.global_prompt()
    return "> "
end

-- line_prompt() — the continuation prompt shown on subsequent input lines.
--
-- Returns:
--   string — e.g. "... "
--
-- The ellipsis is a visual shorthand for "you are in the middle of something".
-- It aligns with the global_prompt width so that columns of code stay tidy.
function DefaultPrompt.line_prompt()
    return "... "
end

return DefaultPrompt
