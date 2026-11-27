---
category: Swift
---

# A throwing FileHandle write can set a precise package-wide macOS floor

The macOS job for PR #20096 (run `45732509599`, job `139652000427`) rejected
`FileHandle.write(contentsOf:)` at three CLI sites because it requires macOS
10.15.4. Replacing it with the older nonthrowing write would hide stdout
failure, while a platform declaration applies to every product in the Swift
package. State the exact minimum in the package spec, manifest, and README, and
be explicit that the package's library product receives the same floor. Keep
other packages and platform targets unchanged.
