plugins {
    kotlin("jvm") version "2.3.21"
    application
}

dependencies {
    implementation("net.java.dev.jna:jna:5.19.1")
    implementation("org.jetbrains.kotlinx:kotlinx-serialization-json:1.11.0")
}

kotlin {
    jvmToolchain(21)
}

application {
    mainClass = "ConformanceKt"
}
