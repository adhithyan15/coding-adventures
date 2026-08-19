// cowsay — routed through paint-vm-ascii (Kotlin port). See
// code/specs/cowsay-paintvm-pipeline.md for the full design rationale.

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

dependencies {
    implementation("com.codingadventures:cli-builder")
    implementation("com.codingadventures:paint-instructions")
    implementation("com.codingadventures:paint-vm-ascii")

    testImplementation(kotlin("test"))
    testImplementation("org.junit.jupiter:junit-jupiter:5.11.4")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
}

tasks.withType<org.jetbrains.kotlin.gradle.tasks.KotlinCompile> {
    compilerOptions {
        jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_21)
    }
}

application {
    mainClass.set("com.codingadventures.cowsay.MainKt")
}

tasks.test {
    useJUnitPlatform()
}
