// app/build.gradle.kts — VisiCalc Android (Jetpack Compose) demo.
//
// Mirrors the visicalc-compose demo (Compose for Desktop) at the
// Composable level — FormulaBar.kt and Grid.kt are byte-identical in
// the two demos because Jetpack Compose for Android and Compose for
// Desktop share the `androidx.compose.*` package and runtime.  Only
// the host (Activity vs. application Window) differs.
//
// When `mosaic-emit-compose` lands, FormulaBar.kt becomes a generated
// file and this demo shifts to the half-generated, half-hand-written
// shape used by visicalc-swiftui / visicalc-qt / visicalc-flutter.

plugins {
    id("com.android.application")
    kotlin("android")
    id("org.jetbrains.kotlin.plugin.compose")
}

// Redirect Gradle's output directory away from `build/` (see
// repo-root lessons.md for the HFS+ collision rationale).
layout.buildDirectory.set(file(".gradle-out"))

android {
    namespace = "com.example.visicalc"
    compileSdk = 35

    defaultConfig {
        applicationId = "com.example.visicalc"
        minSdk = 26   // Android 8.0 — Compose minimum-supported API
        targetSdk = 35
        versionCode = 1
        versionName = "0.1.0"
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    buildFeatures {
        compose = true
    }

    buildTypes {
        debug {
            isMinifyEnabled = false
        }
    }
}

kotlin {
    compilerOptions {
        jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17)
    }
}

dependencies {
    val composeBom = platform("androidx.compose:compose-bom:2024.10.01")
    implementation(composeBom)

    implementation("androidx.core:core-ktx:1.13.1")
    implementation("androidx.activity:activity-compose:1.9.3")
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.material:material")
    implementation("androidx.compose.foundation:foundation")
}
