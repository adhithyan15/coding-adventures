# Changelog

## 0.2.0

- Add permission-probed manual-recording control through the typed D23 camera
  command with exact `camlist.isManRec` readback verification.
- Add PTZ preset recall and five-second-bounded directional movement when both
  the session and camera expose native PTZ support.
- Add explicit native stop, fresh login/logout lifecycle, human approval, and
  loopback protocol coverage for every control path.

## 0.1.0

- Add manual local-HTTPS endpoint intake and Vault-backed Blue Iris credentials.
- Add transport-private `/json` challenge-response authentication and session
  handling.
- Add authorized, bounded server and camera health inspection through `camlist`.
- Add normalized bridge, camera device, entity, capability, and confirmed state
  installation with a real loopback protocol proof.
