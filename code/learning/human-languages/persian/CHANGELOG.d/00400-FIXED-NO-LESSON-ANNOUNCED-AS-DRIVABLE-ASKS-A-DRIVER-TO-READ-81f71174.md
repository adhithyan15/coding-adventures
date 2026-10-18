## Fixed — no lesson announced as drivable asks a driver to read, write or gesture

A lesson whose core is `voice` is announced as "you can do this one in the car", and
only its detachable sections (writing, the letters in a word, script) wait behind the
stop guard. The modality rule now reads the sections a driver hears plainly for a step
that needs eyes or hands (reason `eyes-or-hands-step`, human-language-data); before it,
such steps in the core of a lesson with a letters or writing section were read out at
the wheel.

- **Now not drivable at the core (1), no lesson text changed:** FA-C15-practice ("Write your
  nine letters in a row, right to left, saying each name as you go").

