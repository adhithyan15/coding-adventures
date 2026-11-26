layout.buildDirectory = file("gradle-build")
plugins { kotlin("jvm") version "2.1.20"; `java-library`; jacoco; `maven-publish` }
group = "com.codingadventures"
version = "0.1.0"
repositories { mavenCentral() }
kotlin { jvmToolchain(21) }
dependencies {
    testImplementation(kotlin("test"))
    testImplementation("com.fasterxml.jackson.core:jackson-databind:2.18.3")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
}
tasks.test { useJUnitPlatform(); finalizedBy(tasks.jacocoTestReport) }
jacoco { toolVersion = "0.8.12" }
tasks.jacocoTestReport { dependsOn(tasks.test); reports { xml.required.set(true); html.required.set(true) } }
tasks.jacocoTestCoverageVerification {
    dependsOn(tasks.jacocoTestReport)
    violationRules { rule { limit { counter = "INSTRUCTION"; value = "COVEREDRATIO"; minimum = "0.95".toBigDecimal() } } }
}
tasks.named("check") { dependsOn(tasks.jacocoTestCoverageVerification) }
publishing { publications { create<MavenPublication>("library") { from(components["java"]); artifactId = "rank-statistics-kt" } } }
