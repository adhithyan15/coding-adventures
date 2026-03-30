// app/build.gradle.kts — module-level build script for WaterWear (WearOS).
//
// This file configures:
//   - The Android Gradle Plugin (AGP) with WearOS-specific settings
//   - Kotlin + Jetpack Compose compiler plugin
//   - KSP (Kotlin Symbol Processing) for Room's compile-time SQL generation
//   - All runtime dependencies, including Wear Compose & Room

plugins {
    // Android application plugin — produces an APK or AAB
    alias(libs.plugins.android.application)
    // Kotlin Android plugin — Kotlin support for Android
    alias(libs.plugins.kotlin.android)
    // Kotlin Compose compiler plugin — required since Kotlin 2.0 / AGP 8.3+
    // This replaces the old `compose.kotlinCompilerExtensionVersion` DSL.
    alias(libs.plugins.kotlin.compose)
    // KSP — Kotlin Symbol Processing. Room uses it to generate SQL
    // boilerplate at compile time (faster & type-safe vs Java APT).
    id("com.google.devtools.ksp") version "2.1.0-1.0.29"
}

android {
    namespace = "com.codingadventures.waterwear"

    // compileSdk: the SDK version used to compile. Use latest (35 = Android 15).
    // The watch does not need to run Android 15; this just gives us access to
    // the latest APIs and lint rules.
    compileSdk = 35

    defaultConfig {
        applicationId = "com.codingadventures.waterwear"

        // minSdk = 30: WearOS 3.0 (API 30, released 2021) introduced native
        // Jetpack Compose support on watches. Earlier WearOS versions (1.x / 2.x)
        // used a Tiles/Complications framework — incompatible with Wear Compose.
        // Galaxy Watch 4+, Pixel Watch 1+ all run WearOS 3+, covering >90% of
        // current active WearOS devices.
        minSdk = 30
        targetSdk = 35

        versionCode = 1
        versionName = "1.0.0"
    }

    buildTypes {
        release {
            // Minification disabled for readability in this learning project.
            // A production app should enable R8 to shrink the APK.
            isMinifyEnabled = false
        }
    }

    compileOptions {
        // Java 11 is the minimum JVM bytecode target recommended by Google for
        // modern Android development. It unlocks java.time APIs and lambdas.
        sourceCompatibility = JavaVersion.VERSION_11
        targetCompatibility = JavaVersion.VERSION_11
    }

    kotlinOptions {
        jvmTarget = "11"
    }

    buildFeatures {
        // Enables Jetpack Compose — the declarative UI toolkit.
        // Without this flag, the Compose compiler plugin is not invoked and
        // @Composable annotations are not processed.
        compose = true
    }
}

dependencies {
    // ── Core Android ─────────────────────────────────────────────────────────
    // core-ktx: Kotlin extension functions for Android framework APIs.
    // e.g., context.getSystemService<NotificationManager>() instead of casting.
    implementation(libs.androidx.core.ktx)

    // activity-compose: provides ComponentActivity.setContent { } which is
    // the entry point for a Compose-based screen.
    implementation(libs.androidx.activity.compose)

    // ── Compose BOM (Bill of Materials) ──────────────────────────────────────
    // The BOM pins all androidx.compose.* artifacts to a tested, compatible set
    // of versions. We then reference individual artifacts WITHOUT specifying
    // a version — the BOM provides it.
    implementation(platform(libs.androidx.compose.bom))
    implementation(libs.androidx.ui)
    implementation(libs.androidx.ui.graphics)
    implementation(libs.androidx.ui.tooling.preview)

    // ── Lifecycle ─────────────────────────────────────────────────────────────
    // lifecycle-runtime-ktx: lifecycleScope, repeatOnLifecycle, etc.
    implementation(libs.androidx.lifecycle.runtime.ktx)
    // lifecycle-viewmodel-compose: viewModel() composable factory
    implementation(libs.androidx.lifecycle.viewmodel.compose)

    // ── Wear Compose ─────────────────────────────────────────────────────────
    // IMPORTANT: These are DIFFERENT from the phone Compose Material3 library.
    // Wear Compose Material3 provides watch-optimised widgets:
    //   - Scaffold with a TimeText slot (clock at top edge)
    //   - Button sized for fingertip taps on a 40-45mm screen
    //   - Typography scaled for tiny displays
    // Do NOT mix with androidx.compose.material3 — they have conflicting
    // Scaffold APIs and will cause runtime crashes.
    implementation("androidx.wear.compose:compose-material3:1.0.0-alpha25")

    // compose-foundation: lazy lists, paging, and layout primitives for Wear.
    // Required even if not used directly, because compose-material3 depends on it.
    implementation("androidx.wear.compose:compose-foundation:1.4.0")

    // wear: AlarmManager, WakeupController, and AmbientMode APIs specific to WearOS.
    implementation("androidx.wear:wear:1.3.0")

    // wear-tooling-preview: WearDevices preview annotations for Android Studio.
    // Allows @Preview(device = WearDevices.SMALL_ROUND) annotations.
    implementation("androidx.wear:wear-tooling-preview:1.0.0")

    // ── Room — Local SQLite Database ──────────────────────────────────────────
    // Room is the official SQLite abstraction layer for Android / WearOS.
    // It generates type-safe SQL at compile time via KSP annotations:
    //   @Entity → table definition
    //   @Dao    → SQL queries as suspend functions / Flows
    //   @Database → the database class factory
    val roomVersion = "2.6.1"
    implementation("androidx.room:room-runtime:$roomVersion")
    // room-ktx: adds Flow and coroutines support to Room queries.
    // Without this, @Dao methods return LiveData instead of Flow<T>.
    implementation("androidx.room:room-ktx:$roomVersion")
    // room-compiler: the KSP annotation processor that reads @Entity/@Dao
    // and generates WaterDao_Impl.kt and WaterDatabase_Impl.kt at compile time.
    ksp("androidx.room:room-compiler:$roomVersion")

    // ── Testing ───────────────────────────────────────────────────────────────
    testImplementation(libs.junit)

    // Debug-only: tooling support lets Android Studio render @Preview composables
    debugImplementation(libs.androidx.ui.tooling)
    // ui-test-manifest: merges the test activity into debug APK for Espresso tests
    debugImplementation(libs.androidx.ui.test.manifest)
}
