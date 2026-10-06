# Changelog

## Unreleased

- **Gradle wrapper pins its download.** `gradle-wrapper.properties` gains
  `distributionSha256Sum` (gradle-8.11.1-bin.zip), Gradle's published checksum for the zip it
  names, so the wrapper refuses a download that does not match.
