// Build configuration for the Java IRC server binding.
//
// A thin JNI layer over the Rust `irc_server_native_jni` cdylib, which embeds
// the all-Rust `irc-net-reactor` IRC engine. The cdylib must be built first:
//
//   cargo build --manifest-path ../../rust/Cargo.toml -p irc-server-native-jni --release
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
    // projectDir = code/packages/java/irc-server-native, so
    // projectDir.parentFile.parentFile = code/packages, and the Rust workspace
    // builds into code/packages/rust/target/release.
    val rustReleaseDir = projectDir.parentFile.parentFile
        .resolve("rust/target/release")
        .absolutePath
    jvmArgs("-Djava.library.path=$rustReleaseDir")

    testLogging {
        events("passed", "skipped", "failed")
    }
}
