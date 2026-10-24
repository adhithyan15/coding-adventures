# Collaboration review polls have a ten-second minimum

The agent coordination wait API rejects polling intervals below 10,000 milliseconds. For a quick independent-review check, use the supported ten-second minimum rather than a one-second probe; longer waits remain appropriate when the reviewer is actively inspecting a security-sensitive diff.
