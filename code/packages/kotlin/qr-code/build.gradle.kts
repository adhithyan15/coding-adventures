// Redirect Gradle's output directory away from "build/" to avoid a collision
// with our repo's "BUILD" file on case-insensitive macOS/Windows filesystems.
// See lessons.md: "Gradle 'build' directory conflicts with BUILD file".
layout.buildDirectory = file("gradle-build")

plugins {
    kotlin("jvm") version "2.1.20"
    `java-library`
}

group = "com.codingadventures"
version = "0.1.0"

repositories {
    mavenCentral()
}

tasks.withType<org.jetbrains.kotlin.gradle.tasks.KotlinCompile> {
    compilerOptions {
        jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_21)
    }
}

tasks.withType<JavaCompile> {
    sourceCompatibility = "21"
    targetCompatibility = "21"
    options.release.set(21)
}

dependencies {
    // Local composite builds resolved via settings.gradle.kts includeBuild() declarations.
    // Order: leaf → root (gf256 has no local deps; barcode-2d depends on paint-instructions).
    // paint-instructions is listed explicitly because barcode-2d uses `implementation` (not `api`)
    // for it, so PaintScene is not transitively available to consumers of barcode-2d.
    implementation("com.codingadventures:gf256")
    implementation("com.codingadventures:paint-instructions")
    implementation("com.codingadventures:barcode-2d")

    testImplementation(kotlin("test"))
    testImplementation("org.junit.jupiter:junit-jupiter:5.11.4")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
}

tasks.test {
    useJUnitPlatform()
}
