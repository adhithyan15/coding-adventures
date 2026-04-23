layout.buildDirectory = file("gradle-build")

plugins {
    java
    `java-library`
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
    api("com.codingadventures:hash-map")
    api("com.codingadventures:hash-set")
    api("com.codingadventures:heap")
    api("com.codingadventures:hyperloglog")
    api("com.codingadventures:in-memory-data-store-protocol")
    api("com.codingadventures:radix-tree")
    api("com.codingadventures:skip-list")
    testImplementation("org.junit.jupiter:junit-jupiter:5.11.4")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
}

tasks.test {
    useJUnitPlatform()
}
