// build.gradle.kts — Gradle build script for the Kotlin sql-codegen package.
//
// The layout.buildDirectory override is REQUIRED to avoid a collision between
// Gradle's default "build/" output directory and the "BUILD" script file on
// case-insensitive filesystems (macOS, Windows).  "gradle-build/" is the
// repo-wide convention for Java/Kotlin packages.  (Lesson #48 in lessons.md.)

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

// Pin the JVM target so the compiled class files are compatible with the JDK
// version the CI runner provides via actions/setup-java.
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
    // We consume pre-built JARs from sibling packages rather than wiring up a
    // project() dependency.  The BUILD script pre-compiles both JARs before
    // running tests, ensuring they exist at this path.
    implementation(files("../sql-planner/gradle-build/libs/coding-adventures-sql-planner-0.1.0.jar"))
    implementation(files("../sql-optimizer/gradle-build/libs/coding-adventures-sql-optimizer-0.1.0.jar"))

    testImplementation(kotlin("test"))
    testImplementation("org.junit.jupiter:junit-jupiter:5.11.4")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
}

tasks.test {
    useJUnitPlatform()
    finalizedBy(tasks.jacocoTestReport)
}

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
