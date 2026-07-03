// build.gradle.kts — Gradle build for the Java mini-sqlite Level 1.
//
// IMPORTANT: `layout.buildDirectory` MUST come before the `plugins` block on
// case-insensitive filesystems (macOS, Windows).  If the default `build/`
// directory were used, Gradle would collide with the `BUILD` file at the same
// path, potentially deleting it mid-execution (lesson #48 in lessons.md).
layout.buildDirectory = file("gradle-build")

plugins {
    java
    `java-library`
    jacoco
}

group = "com.codingadventures"
version = "1.0.0"

repositories {
    mavenCentral()
}

// Force Java 21 throughout so that sealed interfaces and record patterns
// (used heavily in the pipeline packages) compile correctly.
tasks.withType<JavaCompile> {
    sourceCompatibility = "21"
    targetCompatibility = "21"
    options.release.set(21)
}

dependencies {
    // Pipeline JARs are consumed via pre-built file references.
    // The BUILD script builds them in leaf-to-root order before `gradle test`.
    //
    // sql-backend  — InMemoryBackend, Row, RowIterator, Cursor, ColumnDef
    // sql-planner  — Statement AST, LogicalPlan, SchemaProvider
    // sql-optimizer — OptimizedPlan
    // sql-codegen  — Program, Instruction hierarchy
    // sql-vm       — SqlVm.execute, QueryResult
    implementation(files("../sql-backend/gradle-build/libs/sql-backend-0.1.0.jar"))
    implementation(files("../sql-planner/gradle-build/libs/coding-adventures-sql-planner-0.1.0.jar"))
    implementation(files("../sql-optimizer/gradle-build/libs/coding-adventures-sql-optimizer-0.1.0.jar"))
    implementation(files("../sql-codegen/gradle-build/libs/coding-adventures-sql-codegen-0.1.0.jar"))
    implementation(files("../sql-vm/gradle-build/libs/coding-adventures-sql-vm-0.1.0.jar"))

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

    // Exclude the Level-0 MiniSqlite class from coverage enforcement.
    // Only Level-1 classes (MiniSqliteConnection, SqlTextParser) are in scope.
    classDirectories.setFrom(
        files(classDirectories.files.map { fileTree(it) {
            exclude("**/MiniSqlite.class", "**/MiniSqlite\$*.class")
        }})
    )

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
