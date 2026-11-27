import org.jetbrains.kotlin.gradle.dsl.JvmTarget

layout.buildDirectory = file("gradle-build")

plugins {
    kotlin("jvm") version "2.1.20"
    `java-library`
}

group = "com.codingadventures"
version = "0.2.0"

repositories {
    mavenCentral()
}

kotlin {
    compilerOptions {
        jvmTarget.set(JvmTarget.JVM_21)
    }
}

java {
    sourceCompatibility = JavaVersion.VERSION_21
    targetCompatibility = JavaVersion.VERSION_21
}

// The canonical replay test stays outside the publishable package, while
// Gradle compiles it against the package API as test-only source.
kotlin.sourceSets.getByName("test").kotlin.srcDir(
    file("../../../specs/fixtures/historical-unix-v1/test-adapters/kotlin"),
)

dependencies {
    testImplementation(kotlin("test"))
    testImplementation("com.fasterxml.jackson.core:jackson-databind:2.18.3")
    testImplementation("org.junit.jupiter:junit-jupiter:5.11.4")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
}

tasks.test {
    useJUnitPlatform()
}
