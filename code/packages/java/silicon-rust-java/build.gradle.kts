// Build configuration for silicon-rust-java — JNI wrapper for the silicon
// simulation Rust crates.
//
// Tests require the Rust cdylib to be built first:
//   cargo build --manifest-path ../../rust/Cargo.toml -p silicon-rust-jni --release
//
// The BUILD script runs that command before invoking `gradle test`.

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

tasks.withType<JavaCompile> {
    sourceCompatibility = "21"
    targetCompatibility = "21"
    options.release.set(21)
}

dependencies {
    testImplementation("org.junit.jupiter:junit-jupiter:5.11.4")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
}

tasks.test {
    useJUnitPlatform()

    // Point the JVM's native library search path at the Rust release build.
    // The executor runs BUILD scripts from the package directory, so
    // projectDir = code/packages/java/silicon-rust-java and
    // projectDir.parentFile.parentFile = code/packages.
    // The Rust workspace builds into code/packages/rust/target/release.
    val rustReleaseDir = projectDir.parentFile.parentFile
        .resolve("rust/target/release")
        .absolutePath
    jvmArgs("-Djava.library.path=$rustReleaseDir")
}
