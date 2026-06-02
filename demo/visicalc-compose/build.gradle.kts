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

kotlin {
    compilerOptions {
        jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17)
    }
}

java {
    sourceCompatibility = JavaVersion.VERSION_17
    targetCompatibility = JavaVersion.VERSION_17
}

compose.desktop {
    application {
        mainClass = "MainKt"

        // Native-distribution packaging (.dmg / .msi / .deb) is
        // intentionally left unconfigured for v0.1.0.  The demo's
        // sole job is to render the window via `./gradlew run`; the
        // packaging story belongs to a follow-up PR.
    }
}
