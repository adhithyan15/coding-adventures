// settings.gradle.kts — single-project Gradle build for the
// VisiCalc Compose for Desktop demo.
//
// We pull the `mosaic-flux-compose` runtime in as an included build
// (Gradle's composite-build feature) so this demo always exercises
// the version of the runtime that lives next to it in the repo, not
// some published artifact.  See:
//   https://docs.gradle.org/current/userguide/composite_builds.html
rootProject.name = "visicalc-compose"

includeBuild("../../../../code/packages/kotlin/mosaic-flux-compose")
