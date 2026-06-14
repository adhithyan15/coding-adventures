// Build configuration for the Kotlin Conduit web framework (WEB10).
//
// This package ships NO new Rust — it reuses the WEB09 `conduit_jni` cdylib
// through a dependency on the Java `conduit` package (pulled in as a Gradle
// composite build; see settings.gradle.kts). Tests still require the cdylib
// to exist, so the BUILD script runs:
//
//   cargo build --manifest-path ../../rust/Cargo.toml -p conduit-jni --release
//
// before `gradle test`.

layout.buildDirectory = file("gradle-build")

plugins {
    kotlin("jvm") version "2.1.20"
    application
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
    // The Java Conduit package — substituted by the composite build. It carries
    // the native cdylib loader and the Application/Server/Request/Response API.
    api("com.codingadventures:conduit")

    testImplementation(kotlin("test"))
    testImplementation("org.junit.jupiter:junit-jupiter:5.11.4")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
}

// Point the JVM's native library search path at the Rust release output so the
// Java package's `System.loadLibrary("conduit_jni")` resolves. projectDir =
// code/packages/kotlin/conduit; up two = code/packages; then rust/target/release.
val rustReleaseDir = projectDir.parentFile.parentFile
    .resolve("rust/target/release")
    .absolutePath

// The bundled 8-route demo (ConduitHello.kt) — run with `gradle run`.
application {
    mainClass.set("com.codingadventures.conduitkt.ConduitHelloKt")
}

tasks.test {
    useJUnitPlatform()
    jvmArgs("-Djava.library.path=$rustReleaseDir")
    testLogging { events("passed", "skipped", "failed") }
}

tasks.named<JavaExec>("run") {
    jvmArgs("-Djava.library.path=$rustReleaseDir")
}
