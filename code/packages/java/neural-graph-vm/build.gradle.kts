layout.buildDirectory = file("gradle-build")

group = "com.codingadventures"
version = "0.1.0"

plugins {
    `java-library`
}

repositories {
    mavenCentral()
}

dependencies {
    implementation("com.codingadventures:neural-network:0.1.0")
    testImplementation("org.junit.jupiter:junit-jupiter:5.11.4")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
}

tasks.test {
    useJUnitPlatform()
}
