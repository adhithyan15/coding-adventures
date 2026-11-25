--- What a campaign found (VER06 section 5, VER06p section 6).
--
-- A report and its crash records are plain tables with a `__tostring` that prints
-- counts and signatures only.  `minimal_input`, `seed` and `corpus` are PRIVATE
-- (VER06 section 6.3): never in a printed form, and public output goes through
-- `Signature.format_tap_failure` only.

-- Standard functions are captured at load so that a target that patches the string
-- metatable or a library table cannot make the engine raise (VER06p section 1).
local setmetatable = setmetatable

local Report = {}

Report.TIER_REPORTED = "caller-reported"
Report.TIER_BLACK_BOX = "black-box-uninstrumented"

local CRASH_META = {}
CRASH_META.__index = CRASH_META

function CRASH_META:__tostring()
  return "#<Fuzz.CrashRecord " .. self.signature .. " " .. self.kind .. " frames=" .. #self.frames .. " hits=" .. self.hits .. ">"
end

--- One deduplicated crash.  `minimal_input` is the shortest input known to reproduce
-- the signature.
function Report.crash_record(fields)
  return setmetatable({
    signature = fields.signature,
    kind = fields.kind,
    frames = fields.frames,
    minimal_input = fields.minimal_input,
    hits = fields.hits,
    minimized = fields.minimized,
    input_dropped = fields.input_dropped,
  }, CRASH_META)
end

local META = {}
META.__index = META

--- Every execution lands in exactly one bucket: `iterations` counts MUTATION
-- executions; crashes/timeouts/ooms count triage and mutation executions; anything
-- that passed is in the corpus or was redundant.
function Report.new(fields)
  return setmetatable({
    iterations = fields.iterations,
    admitted_seeds = fields.admitted_seeds,
    total_edges = fields.total_edges,
    crashes = fields.crashes,
    timeouts = fields.timeouts,
    ooms = fields.ooms,
    crashes_dropped = fields.crashes_dropped,
    admissions_dropped = fields.admissions_dropped,
    leaked = fields.leaked or 0,
    aborted = fields.aborted == true,
    coverage_tier = fields.coverage_tier,
    seed = fields.seed,
    corpus = fields.corpus,
  }, META)
end

--- Can this be read as "the campaign found nothing"?  Only if at least one mutation
-- iteration ran and nothing crashed, timed out, ran out of memory, leaked an
-- execution or was dropped (a crash record, or a corpus admission the retention cap
-- refused: guided exploration that was cut short is not "found nothing").  Zero executions is never a pass.
function Report.clean(report)
  return report.iterations > 0
    and #report.crashes == 0
    and report.timeouts == 0
    and report.ooms == 0
    and report.crashes_dropped == 0
    and report.admissions_dropped == 0
    and report.leaked == 0
end

function Report.unique_crashes(report)
  return #report.crashes + report.crashes_dropped
end

function META:__tostring()
  return "#<Fuzz.Report iterations=" .. self.iterations .. " seeds=" .. self.admitted_seeds .. " edges=" .. self.total_edges
    .. " crashes=" .. Report.unique_crashes(self) .. " timeouts=" .. self.timeouts .. " ooms=" .. self.ooms
    .. " leaked=" .. self.leaked .. " tier=" .. self.coverage_tier .. ">"
end

return Report
