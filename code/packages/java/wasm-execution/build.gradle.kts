layout.buildDirectory = file("gradle-build")

plugins {
    java
    `java-library`
    jacoco
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
    api("com.codingadventures:wasm-leb128")
    api("com.codingadventures:wasm-types")
    testImplementation("org.junit.jupiter:junit-jupiter:5.11.4")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
}

tasks.test {
    useJUnitPlatform()
    finalizedBy(tasks.jacocoTestReport)
}

tasks.jacocoTestReport {
    dependsOn(tasks.test)
    reports {
        xml.required.set(true)
        html.required.set(true)
    }
}
