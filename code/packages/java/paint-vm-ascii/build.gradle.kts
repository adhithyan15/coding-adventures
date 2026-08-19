// Redirect Gradle's output directory away from "build/" to avoid a collision
// with our repo's "BUILD" file on case-insensitive macOS/Windows filesystems.
// See lessons.md: "Gradle 'build' directory conflicts with BUILD file".
layout.buildDirectory = file("gradle-build")

plugins {
    java
    `java-library`
}

group = "com.codingadventures"
version = "0.1.0"

repositories {
    mavenCentral()
}

dependencies {
    // Local paint-instructions package for PaintScene, PaintInstruction, PathCommand.
    // Pulled in via composite build declared in settings.gradle.kts.
    api("com.codingadventures:paint-instructions")

    testImplementation("org.junit.jupiter:junit-jupiter:5.11.4")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
}

tasks.withType<JavaCompile> {
    sourceCompatibility = "21"
    targetCompatibility = "21"
    options.release.set(21)
}

tasks.test {
    useJUnitPlatform()
}
