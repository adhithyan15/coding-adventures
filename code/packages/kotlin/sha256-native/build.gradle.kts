// Kotlin sha256 native binding — JNI over the Rust `sha256_native_jni` cdylib
// (the SAME cdylib used by java/sha256-native; no new Rust crate). Build it:
//   cargo build --manifest-path ../../rust/Cargo.toml -p sha256-native-jni --release

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

dependencies {
    testImplementation(kotlin("test"))
    testImplementation("org.junit.jupiter:junit-jupiter:5.11.4")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
}

tasks.test {
    useJUnitPlatform()
    val rustReleaseDir = projectDir.parentFile.parentFile
        .resolve("rust/target/release")
        .absolutePath
    jvmArgs("-Djava.library.path=$rustReleaseDir")
    testLogging { events("passed", "skipped", "failed") }
}
