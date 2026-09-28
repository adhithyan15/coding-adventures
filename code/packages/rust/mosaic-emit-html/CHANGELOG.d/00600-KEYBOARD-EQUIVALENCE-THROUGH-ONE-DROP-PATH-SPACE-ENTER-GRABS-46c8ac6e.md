- **Keyboard equivalence through one drop path.** Space/Enter grabs, arrows move,
  Space/Enter drops, Escape cancels. Both input methods call the same
  `mosaicCommitDrop`, so the proposal payload is constructed in exactly one
  place and the two cannot drift apart (a test pins the payload to a single
  construction site).
