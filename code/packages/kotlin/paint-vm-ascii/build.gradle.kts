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

dependencies {
    // paint-instructions is a sibling package included via settings.gradle.kts.
    implementation("com.codingadventures:paint-instructions")

    testImplementation(kotlin("test"))
    testImplementation("org.junit.jupiter:junit-jupiter:5.11.4")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
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

tasks.test {
    useJUnitPlatform()
}
