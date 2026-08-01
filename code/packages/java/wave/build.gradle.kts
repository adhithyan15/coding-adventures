layout.buildDirectory = file("gradle-build")

plugins {
    java
}

repositories {
    mavenCentral()
}

dependencies {
    implementation("com.codingadventures:trig")
    testImplementation("org.junit.jupiter:junit-jupiter:5.11.4")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
}

tasks.test {
    useJUnitPlatform()
}
