// app/build.gradle.kts — module-level build configuration.
//
// This is where we wire together:
//   - The Android Gradle Plugin (AGP) which knows how to compile .apk files
//   - The Kotlin compiler
//   - The Compose compiler plugin (separate from Kotlin since Compose 1.5)
//   - KSP (Kotlin Symbol Processing) for Room's compile-time code generation
//
// Room uses KSP instead of KAPT (Kotlin Annotation Processing Tool) because
// KSP is ~2× faster — it works on Kotlin's IR directly rather than going
// through Java stubs.

plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.android)
    alias(libs.plugins.kotlin.compose)
    // KSP version must match the Kotlin version: kotlin-2.1.0 → ksp-2.1.0-1.0.29
    id("com.google.devtools.ksp") version "2.1.0-1.0.29"
}

android {
    namespace = "com.codingadventures.waternotify"
    compileSdk = 35

    defaultConfig {
        applicationId = "com.codingadventures.waternotify"
        // minSdk 26 = Android 8.0 Oreo — the minimum for NotificationChannel.
        // NotificationChannel was introduced in API 26. Below that, channels
        // don't exist and notifications use a simpler (deprecated) API.
        minSdk = 26
        targetSdk = 35
        versionCode = 1
        versionName = "1.0.0"
    }

    buildTypes {
        release {
            // For a learning project we skip minification (ProGuard/R8).
            // In production you'd enable this to shrink the APK.
            isMinifyEnabled = false
        }
    }

    compileOptions {
        // Java 11 bytecode target — required by modern AndroidX libraries.
        // Kotlin compiles to JVM bytecode; this controls which bytecode version.
        sourceCompatibility = JavaVersion.VERSION_11
        targetCompatibility = JavaVersion.VERSION_11
    }

    kotlinOptions {
        jvmTarget = "11"
    }

    buildFeatures {
        // Enables the Compose compiler plugin which transforms @Composable
        // functions into efficient UI tree instructions at compile time.
        compose = true
    }
}

dependencies {
    // AndroidX Core: adds Kotlin extension functions to core Android APIs.
    // e.g. context.getSystemService<AlarmManager>() instead of casting.
    implementation(libs.androidx.core.ktx)

    // Lifecycle runtime: LifecycleOwner, lifecycleScope, repeatOnLifecycle.
    implementation(libs.androidx.lifecycle.runtime.ktx)

    // Activity Compose: setContent { } and ComponentActivity integration.
    implementation(libs.androidx.activity.compose)

    // Compose BOM (Bill of Materials): pins all androidx.compose.* versions
    // to a tested-compatible set so we never accidentally mix mismatched libs.
    implementation(platform(libs.androidx.compose.bom))
    implementation(libs.androidx.ui)
    implementation(libs.androidx.ui.graphics)
    implementation(libs.androidx.ui.tooling.preview)
    implementation(libs.androidx.material3)

    // ViewModel + Compose integration: viewModel() composable factory.
    implementation(libs.androidx.lifecycle.viewmodel.compose)

    // Room — SQLite ORM for Android.
    // room-runtime: the core library (annotations, database, DAO interfaces)
    // room-ktx:     Kotlin coroutines + Flow support (suspend functions in DAO)
    // room-compiler: KSP annotation processor that generates the SQL glue code
    val roomVersion = "2.6.1"
    implementation("androidx.room:room-runtime:$roomVersion")
    implementation("androidx.room:room-ktx:$roomVersion")
    ksp("androidx.room:room-compiler:$roomVersion")

    // Testing
    testImplementation(libs.junit)
    androidTestImplementation(libs.androidx.junit)
    androidTestImplementation(libs.androidx.espresso.core)
    androidTestImplementation(platform(libs.androidx.compose.bom))
    androidTestImplementation(libs.androidx.ui.test.junit4)

    // Debug-only: layout inspector and compose preview
    debugImplementation(libs.androidx.ui.tooling)
    debugImplementation(libs.androidx.ui.test.manifest)
}
