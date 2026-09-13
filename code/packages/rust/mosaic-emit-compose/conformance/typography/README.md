# Compose typography conformance

This authored fixture checks Text, button labels, input placeholders, and
inherited table text/editors against native Compose TextLayoutResult values.
It updates the same composition at 13, 19.5 and 26sp, then checks invalid-value
fallbacks. It also captures typography-200.png for inspection; this is a compiler
fixture, not VisiCalc design acceptance or a pixel-diff gate.

From the repository root (JDK 21 and Gradle required), choose a temporary output:

```powershell
cargo run --manifest-path code/packages/rust/mosaic-compile/Cargo.toml -- pkg code/packages/rust/mosaic-emit-compose/conformance/typography --backend compose --output C:/temp/mosaic-typography --emit-project
New-Item -ItemType Directory -Force C:/temp/mosaic-typography/compose/src/test/kotlin
Copy-Item code/packages/rust/mosaic-emit-compose/conformance/typography/TypographyTest.kt C:/temp/mosaic-typography/compose/src/test/kotlin/
gradle --no-daemon -p C:/temp/mosaic-typography/compose test --tests TypographyTest
```

No Rust runtime library is needed: this fixture directly composes the generated
component. Inspect the Gradle test report and screenshot in the output project.
