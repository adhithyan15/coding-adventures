// build.gradle.kts — VisiCalc demo on Compose for Desktop (JVM).
//
// Sixth cross-backend visual demo (Phase 2 / VC2-compose).  Mirrors
// the React / HTML / WebComponent / SwiftUI / Qt / Flutter / XAML
// visuals: dark theme, 5×5 sample dataset hard-coded, A1 selected,
// formula bar showing "=SUM(B1:B5)".
//
// Unlike its siblings, this demo is **entirely hand-written** for
// v0.1.0 — no `mosaic-compile --backend compose` exists yet.  The
// FormulaBar and Grid composables live in `src/main/kotlin/` and
// visually match what the eventual Compose emitter should produce.
// When `mosaic-emit-compose` lands, the FormulaBar will be replaced
// by `src/main/kotlin/generated/FormulaBar.kt` and this demo will
// shift to a half-generated, half-hand-written shape like SwiftUI.
//
// Compose for Desktop targets the JVM; `./gradlew run` opens a
// native window via `Window { ... }` from `androidx.compose.ui.window`.

plugins {
    kotlin("jvm") version "2.0.0"
    id("org.jetbrains.compose") version "1.6.11"
    id("org.jetbrains.kotlin.plugin.compose") version "2.0.0"
}

group = "org.mosaic.demo"
version = "0.1.0"

// Redirect Gradle's output directory away from `build/` because the
// repo's case-insensitive filesystem (macOS HFS+) treats `build` and
// the required `BUILD` script as the same name.  This is the same
// trap that bit mosaic-flux-compose (see lessons.md).
layout.buildDirectory.set(file(".gradle-out"))

repositories {
    mavenCentral()
    google()
    maven("https://maven.pkg.jetbrains.space/public/p/compose/dev")
}

dependencies {
    implementation(compose.desktop.currentOs)
    implementation("org.mosaic.flux:mosaic-flux-compose:0.1.0")
}

// Target JDK 21: the engine is reached through the Java Foreign Function &
// Memory API (Engine.kt), which is preview on JDK 21 and final on JDK 22.
kotlin {
    compilerOptions {
        jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_21)
    }
}

java {
    sourceCompatibility = JavaVersion.VERSION_21
    targetCompatibility = JavaVersion.VERSION_21
}

compose.desktop {
    application {
        mainClass = "MainKt"

        // The demo computes on the Rust engine through the Java FFM API (see
        // src/main/kotlin/Engine.kt). FFM is a preview feature on JDK 21, so
        // pass --enable-preview at run time; --enable-native-access silences the
        // restricted-method warning. (Both become unnecessary on JDK 22+, where
        // FFM is final.) Engine.kt loads native/libspreadsheet_capi.* relative
        // to the run directory.
        jvmArgs += listOf("--enable-preview", "--enable-native-access=ALL-UNNAMED")

        // Native-distribution packaging so the demo can be built as a real,
        // double-clickable app bundle (`gradle createDistributable` →
        // build/compose/binaries/main/app/VisiCalc.app) that holds its window —
        // unlike `gradle run`, which doesn't keep a GUI session when launched
        // non-interactively. The engine's `libspreadsheet_capi.*` from `native/`
        // is bundled as an app resource; Engine.kt's resolver checks the packaged
        // resources dir (the `compose.application.resources.dir` system property
        // jpackage sets) so the FFM lookup works from inside the bundle too.
        nativeDistributions {
            targetFormats(
                org.jetbrains.compose.desktop.application.dsl.TargetFormat.Dmg,
            )
            packageName = "VisiCalc"
            // jpackage rejects a leading-zero version for the macOS app image.
            packageVersion = "1.0.0"
            // Bundle the engine dylib. Compose expects this dir to hold per-target
            // subdirs (`common`, `macos-arm64`, `macos-x64`, `windows-x64`,
            // `linux-x64`); at runtime the current target's files are flattened
            // into the `compose.application.resources.dir` directory, where
            // Engine.kt's resolver looks. scripts/build.sh populates the right
            // subdir from the freshly built capi (git-ignored, like native/).
            appResourcesRootDir.set(project.layout.projectDirectory.dir("appResources"))
        }
    }
}
