// cowsay — routed through paint-vm-ascii (Java port). See
// code/specs/cowsay-paintvm-pipeline.md for the full design rationale.

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

dependencies {
    implementation("com.codingadventures:cli-builder")
    implementation("com.codingadventures:paint-instructions")
    implementation("com.codingadventures:paint-vm-ascii")

    testImplementation("org.junit.jupiter:junit-jupiter:5.11.4")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
}

tasks.withType<JavaCompile> {
    sourceCompatibility = "21"
    targetCompatibility = "21"
    options.release.set(21)
}

application {
    mainClass.set("com.codingadventures.cowsay.Main")
}

tasks.test {
    useJUnitPlatform()
}
