## Unreleased — voice mode carries the accepted answers the generator emits

- **Fixed:** `acceptedOf` in `src/voicescript.ts` read only the old hand-built
  `answer` + `accepted` fields, but the narration generator emits a scored
  activity's answers as `acceptedResponses` — the only shape in the committed
  narration JSON. Every `respond` step built from real narration therefore
  carried no accepted answers. It now reads `acceptedResponses` first
  (canonical first), still accepts the old fields, trims, and drops repeats.
- Nothing plays these answers yet (the player waits the response budget and
  moves on); they are the contract a future speech-recognition step reads.
- **Hardened:** a malformed `acceptedResponses`, `accepted` or `assesses` (an
  object, a number, a bare string) is ignored instead of throwing "not
  iterable" and stopping voice mode for the lesson, on both the respond path
  and the no-verdict fallback.
