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
    // GF(256) field arithmetic — MA01 package in this monorepo.
    // Pulled in via composite build declared in settings.gradle.kts.
    api("com.codingadventures:gf256")

    // barcode-2d provides ModuleGrid, ModuleShape, Barcode2DLayoutConfig,
    // and the layout() function.  Pulled in via composite build.
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
