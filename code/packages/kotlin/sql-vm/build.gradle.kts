// build.gradle.kts — Gradle build script for the Kotlin sql-vm package.
//
// CRITICAL: layout.buildDirectory MUST be redirected away from "build/" to
// avoid a collision with the "BUILD" script file on case-insensitive
// filesystems (macOS HFS+, Windows NTFS).  "gradle-build/" is the repo-wide
// convention for all Java/Kotlin packages.  (lessons.md §BUILD files, lesson #48.)
//
// Dependency strategy: we consume pre-built JARs from sibling packages rather
// than using Gradle's project() dependencies.  The BUILD script pre-compiles all
// sibling JARs before running this package's tests, so the files exist at their
// declared paths when Gradle resolves them.

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

// Pin the JVM target to match the CI runner's actions/setup-java configuration.
// We deliberately do NOT use java { toolchain { languageVersion } } — that would
// ask Gradle to download a JDK, which fails on restricted CI runners.  Letting
// Gradle use the running JDK (provided by setup-java) is the correct approach.
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
    // Four sibling JARs, pre-built by the BUILD script in leaf-to-root order.
    // These paths are relative to this package's directory.
    // Note: sql-backend's rootProject.name is "sql-backend" (no "coding-adventures-" prefix);
    // the other three packages include the prefix.
    implementation(files("../sql-backend/gradle-build/libs/sql-backend-0.1.0.jar"))
    implementation(files("../sql-planner/gradle-build/libs/coding-adventures-sql-planner-0.1.0.jar"))
    implementation(files("../sql-optimizer/gradle-build/libs/coding-adventures-sql-optimizer-0.1.0.jar"))
    implementation(files("../sql-codegen/gradle-build/libs/coding-adventures-sql-codegen-0.1.0.jar"))

    testImplementation(kotlin("test"))
    testImplementation("org.junit.jupiter:junit-jupiter:5.11.4")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
}

tasks.test {
    useJUnitPlatform()
    finalizedBy(tasks.jacocoTestReport)
}

// JaCoCo coverage configuration.
// Minimum coverage threshold is 80% line coverage (lessons.md §Repo Standards).
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
                counter = "LINE"
                value = "COVEREDRATIO"
                minimum = "0.80".toBigDecimal()
            }
        }
    }
}

tasks.named("check") {
    dependsOn(tasks.jacocoTestCoverageVerification)
}
