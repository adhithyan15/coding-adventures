// app/build.gradle.kts
// Build configuration for the Android app module.
//
// Gradle is Android's build system — equivalent to Xcode's build
// settings, but written in Kotlin (or Groovy). The .kts extension
// means this is Kotlin Script, which gives you type safety and
// IDE autocomplete in the build file itself.

plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.android)
    alias(libs.plugins.kotlin.compose)
}

android {
    namespace = "com.codingadventures.helloworld"

    // compileSdk is the Android API level used to compile the app.
    // API 36 = Android 16 (2026). Always compile against the latest.
    compileSdk = 36

    defaultConfig {
        applicationId = "com.codingadventures.helloworld"

        // minSdk is the oldest Android version the app supports.
        // API 26 = Android 8.0 (Oreo, 2017). Covers ~95% of devices.
        minSdk = 26

        // targetSdk tells Android which behaviour version to apply.
        // Should always match compileSdk.
        targetSdk = 36

        versionCode = 1
        versionName = "1.0.0"
    }

    buildTypes {
        release {
            isMinifyEnabled = true   // shrinks and obfuscates the release build
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt")
            )
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_21
        targetCompatibility = JavaVersion.VERSION_21
    }

    kotlinOptions {
        jvmTarget = "21"
    }

    buildFeatures {
        // Enable Jetpack Compose
        compose = true
    }
}

dependencies {
    // Jetpack Compose BOM (Bill of Materials) pins all Compose library
    // versions so they stay compatible with each other.
    val composeBom = platform(libs.androidx.compose.bom)
    implementation(composeBom)

    implementation(libs.androidx.core.ktx)
    implementation(libs.androidx.lifecycle.runtime.ktx)
    implementation(libs.androidx.activity.compose)
    implementation(libs.androidx.compose.ui)
    implementation(libs.androidx.compose.ui.tooling.preview)
    implementation(libs.androidx.compose.material3)
    implementation(libs.androidx.compose.material.icons.extended)

    debugImplementation(libs.androidx.compose.ui.tooling)
}
