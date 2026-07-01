// layout.buildDirectory must be FIRST — before the plugins block.
// On Windows and macOS the filesystem is case-insensitive, so Gradle's default
// `build/` output directory would collide with our uppercase `BUILD` script.
// Moving the output to `gradle-build/` sidesteps that collision entirely.
layout.buildDirectory = file("gradle-build")

plugins {
    kotlin("jvm") version "2.1.20"
    `java-library`
    jacoco
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

tasks.withType<JavaCompile> {
    sourceCompatibility = "21"
    targetCompatibility = "21"
    options.release.set(21)
}

dependencies {
    // Pre-built JARs from sibling packages (built by BUILD/BUILD_windows scripts).
    // Note: sql-backend uses rootProject.name = "sql-backend" (no prefix);
    // the other packages include "coding-adventures-" prefix in the JAR name.
    implementation(files("../sql-backend/gradle-build/libs/sql-backend-0.1.0.jar"))
    implementation(files("../sql-planner/gradle-build/libs/coding-adventures-sql-planner-0.1.0.jar"))
    implementation(files("../sql-optimizer/gradle-build/libs/coding-adventures-sql-optimizer-0.1.0.jar"))
    implementation(files("../sql-codegen/gradle-build/libs/coding-adventures-sql-codegen-0.1.0.jar"))
    implementation(files("../sql-vm/gradle-build/libs/coding-adventures-sql-vm-0.1.0.jar"))
    testImplementation(kotlin("test"))
    testImplementation("org.junit.jupiter:junit-jupiter:5.11.4")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
}

tasks.test {
    useJUnitPlatform()
}

// ── JaCoCo configuration ──────────────────────────────────────────────────────
//
// JaCoCo instruments bytecode at test time to track which lines and branches
// are executed.  The two tasks below produce XML and HTML reports, and enforce
// an 80% instruction coverage floor.  CI fails if coverage drops below that.

jacoco {
    toolVersion = "0.8.12"
}

tasks.jacocoTestReport {
    dependsOn(tasks.test)
    reports {
        xml.required.set(true)
        html.required.set(true)
    }
}

tasks.jacocoTestCoverageVerification {
    dependsOn(tasks.jacocoTestReport)
    violationRules {
        rule {
            limit {
                minimum = "0.80".toBigDecimal()
            }
        }
    }
}
