### Fixed — generated Android projects verify the Gradle distribution they download (UI89)

- The generated Android project's `gradle/wrapper/gradle-wrapper.properties`
  now carries `distributionSha256Sum`, the SHA-256 of the
  `gradle-8.14.3-bin.zip` it names. The value is Gradle's published checksum
  (https://gradle.org/release-checksums/), held in the new
  `ANDROID_GRADLE_DISTRIBUTION_SHA256` next to `ANDROID_GRADLE_VERSION`; bump
  both together. Without it, the wrapper would download and run any zip served
  at that URL. With it, the wrapper refuses a download that does not match. The
  Android project test pins the line.
