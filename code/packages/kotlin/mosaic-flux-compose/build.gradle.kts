// build.gradle.kts — mosaic-flux-compose v0.1.0.
//
// Kotlin runtime for Mosaic UI's Jetpack Compose emitter.  Mirrors
// the API surface of mosaic-flux-react / html / webcomponent /
// swiftui in idiomatic Kotlin: interface MosaicAction with apply(),
// generic MosaicStore<S> exposing kotlinx.coroutines.flow.StateFlow<S>
// for fine-grained Compose `collectAsState` integration.

plugins {
    kotlin("jvm") version "2.0.0"
}

group = "org.mosaic.flux"
version = "0.1.0"

// Redirect Gradle's output directory away from "build/" because the
// repo's case-insensitive filesystem treats it as the same name as
// our required "BUILD" file (the build-tool script).  Using a
// dot-prefixed name also keeps gradle output out of git ls-files
// listings.
layout.buildDirectory.set(file(".gradle-out"))

repositories {
    mavenCentral()
}

dependencies {
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.9.0")
    testImplementation(kotlin("test"))
    testImplementation("org.jetbrains.kotlinx:kotlinx-coroutines-test:1.9.0")
    testImplementation("org.junit.jupiter:junit-jupiter:5.10.0")
}

tasks.test {
    useJUnitPlatform()
}

kotlin {
    // JVM 17 is the minimum but the CI environment may have only JDK
    // 21; we use jvmTarget=17 with whatever JDK is available rather
    // than requiring a toolchain download.
    compilerOptions {
        jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17)
    }
}

java {
    sourceCompatibility = JavaVersion.VERSION_17
    targetCompatibility = JavaVersion.VERSION_17
}
