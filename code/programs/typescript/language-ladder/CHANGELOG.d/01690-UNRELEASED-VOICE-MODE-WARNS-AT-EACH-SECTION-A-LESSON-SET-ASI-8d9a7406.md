## Unreleased — voice mode warns at each section a lesson set aside

`buildVoiceScript` speaks `STOP_GUARD_SPEECH` ("Once you have stopped driving:
this part needs your eyes or your hands. If you are driving, skip ahead to the
next part.") straight after the title of every section listed in the lesson
notice's `waitUntilStopped`, before any of its content. The notice tells a driver
"I will say so again when we reach it"; voice mode now does, matching the
narration text.
