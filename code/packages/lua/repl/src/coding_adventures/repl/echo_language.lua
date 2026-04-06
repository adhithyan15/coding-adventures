-- echo_language — A trivial Language plug-in that echoes input back
--
-- # What Is a Language Plug-in?
--
-- The REPL framework is deliberately decoupled from any specific programming
-- language. Instead of hard-coding "evaluate Python" or "evaluate Lua", it
-- accepts a *Language* object at runtime. A Language object is a plain Lua
-- table that must expose one method:
--
--   language.eval(input) → result
--
-- The result must be one of three shapes (tagged unions, sometimes called
-- "variant" or "sum types"):
--
--   { tag = "ok",    output = string_or_nil }   -- success, optional text
--   { tag = "error", message = string        }   -- evaluation failed
--   { tag = "quit"                           }   -- user wants to exit
--
-- This tagged-union style is idiomatic Lua: because Lua lacks algebraic data
-- types, we use a "tag" field to discriminate between cases.
--
-- # Why EchoLanguage?
--
-- EchoLanguage is the simplest possible conforming Language. It serves two
-- purposes:
--
--   1. Demonstration — A new reader can see the complete Language contract
--      without wading through parser or interpreter code.
--
--   2. Testing — The REPL loop's control flow (prompt → read → eval → print →
--      loop) can be exercised without a real language engine.
--
-- # Behavior
--
--   ":quit"   → { tag = "quit" }            user typed the quit command
--   anything  → { tag = "ok", output = x }  echo the input as output

local EchoLanguage = {}

-- The quit sentinel string. Externalising it lets tests reference the same
-- constant rather than duplicating a string literal.
EchoLanguage.QUIT_COMMAND = ":quit"

-- eval(input) — evaluate a single line of user input.
--
-- Parameters:
--   input (string) — the raw line the user typed (trailing newline stripped)
--
-- Returns:
--   A result table. See module header for the three possible shapes.
--
-- Design note: We compare with == rather than pattern matching because the
-- quit command is an exact, case-sensitive keyword. A real language would
-- tokenise and parse instead.
function EchoLanguage.eval(input)
    -- Guard: input should always be a string, but be defensive.
    if type(input) ~= "string" then
        return { tag = "error", message = "input must be a string" }
    end

    -- The only special command: ":quit" signals the REPL to stop the loop.
    if input == EchoLanguage.QUIT_COMMAND then
        return { tag = "quit" }
    end

    -- For any other input, echo it back verbatim.
    -- output may be an empty string if the user just pressed Enter.
    return { tag = "ok", output = input }
end

return EchoLanguage
