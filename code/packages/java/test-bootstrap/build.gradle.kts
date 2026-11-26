layout.buildDirectory = file("gradle-build")

plugins {
    java
    `java-library`
    jacoco
}

group = "com.codingadventures"
version = "0.4.3"

repositories {
    mavenCentral()
}

tasks.withType<JavaCompile> {
    sourceCompatibility = "21"
    targetCompatibility = "21"
    options.release.set(21)
    options.encoding = "UTF-8"
}

dependencies {
    testImplementation("org.junit.jupiter:junit-jupiter:5.11.4")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
}

tasks.test {
    useJUnitPlatform()
    finalizedBy(tasks.jacocoTestReport)
}

jacoco { toolVersion = "0.8.14" }

tasks.jacocoTestReport {
    dependsOn(tasks.test)
    reports { xml.required.set(true); html.required.set(true) }
}

tasks.jacocoTestCoverageVerification {
    dependsOn(tasks.jacocoTestReport)
    violationRules {
        rule {
            limit {
                counter = "LINE"
                value = "COVEREDRATIO"
                minimum = "0.95".toBigDecimal()
            }
        }
    }
}

tasks.named("check") { dependsOn(tasks.jacocoTestCoverageVerification) }
