### Fixed — keep page-following and bare paired reading out of voice cores

- Treat a spoken instruction to follow a printed line (including with a finger),
  read both lines, or listen and then read as requiring sight or hands. A planted
  version of each step now revokes a committed Kannada A1 lesson's voice core
  before its narration can announce the lesson as drivable.
- Keep the detection bounded to narrated steps and explicit page objects. Audio
  instructions, quoted examples, deferred reading cues, and interpretations of
  a heard line remain outside this safety rule.
- Regenerated the modality and narration outputs: neither committed corpus
  artifact changed. The new cases cover future authored lessons and cues.
