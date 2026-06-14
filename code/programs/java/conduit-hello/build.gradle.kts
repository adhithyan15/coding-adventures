// Demo program for the Java Conduit framework — an 8-route Sinatra-style app.
//
// Depends on the `conduit` package via a Gradle composite build (see
// settings.gradle.kts). Tests require the Rust cdylib built first:
//
//   cargo build --manifest-path ../../../packages/rust/Cargo.toml -p conduit-jni --release

layout.buildDirectory = file("gradle-build")

plugins {
    java
    application
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
    implementation("com.codingadventures:conduit")
    testImplementation("org.junit.jupiter:junit-jupiter:5.11.4")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
}

application {
    mainClass.set("com.codingadventures.conduithello.ConduitHello")
}

// java.library.path must point at the Rust release output so the cdylib loads.
// projectDir = code/programs/java/conduit-hello; up three = code; then
// packages/rust/target/release.
val rustReleaseDir = projectDir.parentFile.parentFile.parentFile
    .resolve("packages/rust/target/release")
    .absolutePath

tasks.test {
    useJUnitPlatform()
    jvmArgs("-Djava.library.path=$rustReleaseDir")
    testLogging { events("passed", "skipped", "failed") }
}

tasks.named<JavaExec>("run") {
    jvmArgs("-Djava.library.path=$rustReleaseDir")
}
