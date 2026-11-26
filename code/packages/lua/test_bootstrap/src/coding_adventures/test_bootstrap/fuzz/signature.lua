--- Crash signatures, frame normalisation and the one public failure line
-- (VER06 sections 5.2 and 6, VER06p sections 5.6 and 7).

-- Standard functions are captured at load so that a target that patches the string
-- metatable or a library table cannot make the engine raise (VER06p section 1).
local m_type = math.type
local s_sub, s_gsub, s_find, s_format, s_match = string.sub, string.gsub, string.find, string.format, string.match
local t_concat = table.concat
local dgetinfo = debug.getinfo
local type, error, rawequal, rawlen, rawget, ipairs = type, error, rawequal, rawlen, rawget, ipairs

local Int = require("coding_adventures.test_bootstrap.int")
local Prng = require("coding_adventures.test_bootstrap.prng")
local Limits = require("coding_adventures.test_bootstrap.fuzz.limits")
local FailureIds = require("coding_adventures.test_bootstrap.fuzz.failure_ids")
-- captured at load: a target that rewrites Limits later changes nothing here
local limits_MAX_TARGET_LEN, limits_MAX_KIND_LEN, limits_MAX_BASENAME, limits_MAX_FRAMES, limits_MAX_WALK, limits_MAX_WALK_TEXT = Limits.MAX_TARGET_LEN, Limits.MAX_KIND_LEN, Limits.MAX_BASENAME, Limits.MAX_FRAMES, Limits.MAX_WALK, Limits.MAX_WALK_TEXT
-- captured at load: a target that rewrites FailureIds later changes nothing here
local failureids_is_fuzz_failure_id, failureids_CRASH, failureids_TIMEOUT, failureids_OOM = FailureIds.is_fuzz_failure_id, FailureIds.CRASH, FailureIds.TIMEOUT, FailureIds.OOM
-- captured at load: a target that rewrites Prng later changes nothing here
local prng_fnv1a64_word = Prng.fnv1a64_word
-- captured at load: a target that rewrites Int later changes nothing here
local int_is = Int.is

local signature_crash_signature, signature_duplicate_crash, signature_token, signature_format_tap_failure, signature_tap_failures, signature_normalise_kind, signature_normalise_frame, signature_collapse, signature_frames_of
local Signature = {}

--- The low 32 bits of fnv1a64(kind ":" frames joined by ":"), 8 lowercase hex.
--
-- Frames are NOT escaped (VER06 section 9.4 item 6): one frame "x:y" and two frames
-- "x", "y" hash alike.  This lane never produces a frame with a colon
-- (`normalise_frame`), so the ambiguity cannot arise from Lua targets.
function signature_crash_signature(kind, frames)
  if type(kind) ~= "string" then
    error("kind must be a string", 2)
  end
  if type(frames) ~= "table" then
    error("frames must be a list of strings", 2)
  end
  local n = rawlen(frames)
  local parts = {}
  for i = 1, n do
    local f = rawget(frames, i)
    if type(f) ~= "string" then
      error("frames must be a list of strings", 2)
    end
    parts[i] = f
  end
  return s_format("%08x", prng_fnv1a64_word(kind .. ":" .. t_concat(parts, ":")) & 0xFFFFFFFF)
end

function signature_duplicate_crash(kind_a, frames_a, kind_b, frames_b)
  return signature_crash_signature(kind_a, frames_a) == signature_crash_signature(kind_b, frames_b)
end

--- The bytes of `text` reduced to the grammar [A-Za-z0-9._/-]: truncated to `limit`
-- BYTES first (so a cut in the middle of a multi-byte character leaves one `_`), then
-- every other BYTE (a control character, a newline, one byte of a multi-byte
-- character) becomes `_`.
function signature_token(text, limit)
  return (s_gsub(s_sub(text, 1, limit), "[^A-Za-z0-9._/%-]", "_"))
end

--- The only public failure line: `not ok N - target [failure-id:hex8]`.  It carries no
-- input bytes, seed, message or stack trace (VER00 7.3).
function signature_format_tap_failure(test_num, target, failure_id, sig_hash)
  if not int_is(test_num) then
    error("test_num must be an Integer", 2)
  end
  if m_type(test_num) ~= "integer" or test_num < 1 or test_num > 2147483647 then
    error("test_num must be in 1..2147483647", 2)
  end
  if type(target) ~= "string" then
    error("target must be a string", 2)
  end
  if not failureids_is_fuzz_failure_id(failure_id) then
    error("failure id is not one of the registered fuzz failure ids", 2)
  end
  if type(sig_hash) ~= "string" or #sig_hash ~= 8 or s_find(sig_hash, "[^0-9a-f]") then
    error("sig hash must be exactly 8 lowercase hex digits", 2)
  end
  return s_format("not ok %d - %s [%s:%s]", test_num, signature_token(target, limits_MAX_TARGET_LEN), failure_id, sig_hash)
end

--- A report as public, opaque failure lines (none for a clean one): one per crash
-- signature, then one for timeouts, one for OOMs and one for leaked executions, whose
-- hash is the signature of the bare kind (they have no stack to hash).
function signature_tap_failures(report, target, first_test)
  first_test = first_test or 1
  local lines = {}
  for i = 1, #report.crashes do
    lines[#lines + 1] = signature_format_tap_failure(first_test + i - 1, target, failureids_CRASH, report.crashes[i].signature)
  end
  local extra = {
    { report.timeouts, failureids_TIMEOUT, "timeout" },
    { report.ooms, failureids_OOM, "oom" },
    { report.leaked or 0, failureids_TIMEOUT, "executions-leaked" },
  }
  for _, e in ipairs(extra) do
    if e[1] > 0 then
      lines[#lines + 1] = signature_format_tap_failure(first_test + #lines, target, e[2], signature_crash_signature(e[3], {}))
    end
  end
  return lines
end

--- A crash kind from a name: anything outside [A-Za-z0-9._<>$/-] becomes `_`, at
-- most 64 characters.  (The engine's own kinds are `type()` results and fixed words;
-- this is for callers that build a kind from other text.)
function signature_normalise_kind(name)
  if type(name) ~= "string" then
    return "<anonymous>"
  end
  return s_sub((s_gsub(name, "[^A-Za-z0-9._<>$/%-]", "_")), 1, limits_MAX_KIND_LEN)
end

--- `basename#line`, every character outside [A-Za-z0-9._<>$/#-] made `_`, so no frame
-- contains `:`.
function signature_normalise_frame(path, line)
  local base = s_match(path, "([^/\\]*)$") or path
  base = s_sub(base, 1, limits_MAX_BASENAME)
  return (s_gsub(base .. "#" .. line, "[^A-Za-z0-9._<>$/#%-]", "_"))
end

--- Drops consecutive duplicates (a recursive crash hashes alike at any depth) and
-- keeps the innermost MAX_FRAMES.
function signature_collapse(frames)
  local out = {}
  for _, f in ipairs(frames) do
    if out[#out] ~= f then
      out[#out + 1] = f
      if #out >= limits_MAX_FRAMES then
        break
      end
    end
  end
  return out
end

-- The directory of the engine's own files: a frame inside it is plumbing, never "the
-- target".  Computed once from this file's own chunk name (nil when it was not loaded
-- from a file, in which case nothing is excluded).
local ENGINE_DIR = (function()
  local source = dgetinfo(1, "S").source
  return s_match(source, "^@(.*[/\\])fuzz[/\\][^/\\]*$")
end)()

local function is_engine_source(source)
  return ENGINE_DIR ~= nil and s_sub(source, 1, 1) == "@" and s_sub(source, 2, 1 + #ENGINE_DIR) == ENGINE_DIR
end

--- The frames of a coroutine that ended in an error, innermost first, from its
-- intact stack (an errored coroutine keeps it).  C functions, the engine's own files
-- and everything from the executor's `body` frame outwards are dropped; consecutive
-- duplicates collapse; at most MAX_WALK levels and MAX_WALK_TEXT bytes of chunk name
-- are inspected.  No target value is read, called or compared with a metamethod.
function signature_frames_of(co, body)
  local frames, text = {}, 0
  for level = 0, limits_MAX_WALK - 1 do
    local info = dgetinfo(co, level, "Slf")
    if info == nil or rawequal(info.func, body) then
      break
    end
    if info.what ~= "C" then
      local source = info.source
      text = text + #source
      if text > limits_MAX_WALK_TEXT then
        break
      end
      if not is_engine_source(source) then
        frames[#frames + 1] = signature_normalise_frame(info.short_src, info.currentline)
      end
    end
  end
  return signature_collapse(frames)
end

-- the public table is for callers; this module calls its own locals
Signature.crash_signature = signature_crash_signature
Signature.duplicate_crash = signature_duplicate_crash
Signature.token = signature_token
Signature.format_tap_failure = signature_format_tap_failure
Signature.tap_failures = signature_tap_failures
Signature.normalise_kind = signature_normalise_kind
Signature.normalise_frame = signature_normalise_frame
Signature.collapse = signature_collapse
Signature.frames_of = signature_frames_of

return Signature
