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
    // Local composite builds resolved via settings.gradle.kts includeBuild() declarations.
    // Order: leaf → root (paint-instructions has no local deps; barcode-2d depends on it).
    // paint-instructions is listed explicitly because barcode-2d uses `implementation` (not `api`)
    // for it, so PaintScene is not transitively available to consumers of barcode-2d.
    api("com.codingadventures:paint-instructions")
    api("com.codingadventures:barcode-2d")

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
