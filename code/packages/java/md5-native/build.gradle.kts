// Java md5 native binding — JNI over the Rust `md5_native_jni` cdylib.
// Build the cdylib first:
//   cargo build --manifest-path ../../rust/Cargo.toml -p md5-native-jni --release

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
    val rustReleaseDir = projectDir.parentFile.parentFile
        .resolve("rust/target/release")
        .absolutePath
    jvmArgs("-Djava.library.path=$rustReleaseDir")
    testLogging { events("passed", "skipped", "failed") }
}
