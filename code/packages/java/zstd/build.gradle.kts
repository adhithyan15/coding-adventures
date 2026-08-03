layout.buildDirectory = file("gradle-build")

plugins {
    java
    `java-library`
    jacoco
}

group = "com.codingadventures"
version = "0.1.1"

repositories {
    mavenCentral()
}

tasks.withType<JavaCompile> {
    sourceCompatibility = "21"
    targetCompatibility = "21"
    options.release.set(21)
}

dependencies {
    api("com.codingadventures:lzss")
    testImplementation("org.junit.jupiter:junit-jupiter:5.11.4")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
}

tasks.test {
    useJUnitPlatform()
    finalizedBy(tasks.jacocoTestReport)

    // One test (testDecompressRejectsOversizedMultiBlockExpansion) legitimately
    // accumulates output close to Zstd.MAX_OUTPUT (256 MB) before the
    // decompression-bomb guard fires on the block that tips it over — that's
    // the guard doing its job, not a leak, but it needs real heap to get
    // there. Gradle's default test-worker heap is too small for that single
    // test without OOMing before the guard has a chance to run (confirmed
    // empirically). 2 GB comfortably covers the ~256-400 MB peak (including
    // ByteBuf's 1.5x growth overshoot) plus JVM/test-framework overhead.
    maxHeapSize = "2g"
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
