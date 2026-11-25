layout.buildDirectory = file("gradle-build")
plugins { java; `java-library`; application; jacoco; `maven-publish` }
group = "com.codingadventures"
version = "0.1.0"
repositories { mavenCentral() }
java { toolchain { languageVersion.set(JavaLanguageVersion.of(21)) } }
dependencies {
    api("com.codingadventures:rank-statistics:0.1.0")
    implementation("com.fasterxml.jackson.core:jackson-databind:2.18.3")
    testImplementation("org.junit.jupiter:junit-jupiter:5.11.4")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
}
application { mainClass.set("com.codingadventures.microbenchmark.ArrayMicrobenchmark") }
tasks.test { useJUnitPlatform(); finalizedBy(tasks.jacocoTestReport) }
jacoco { toolVersion = "0.8.12" }
tasks.jacocoTestReport { dependsOn(tasks.test); reports { xml.required.set(true); html.required.set(true) } }
tasks.jacocoTestCoverageVerification {
    dependsOn(tasks.jacocoTestReport)
    violationRules { rule { limit { counter = "INSTRUCTION"; value = "COVEREDRATIO"; minimum = "0.95".toBigDecimal() } } }
}
tasks.named("check") { dependsOn(tasks.jacocoTestCoverageVerification) }
publishing { publications { create<MavenPublication>("library") { from(components["java"]); artifactId = "microbenchmark" } } }
