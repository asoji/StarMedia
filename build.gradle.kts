plugins {
    `java`
    alias(libs.plugins.kotlin)
    alias(libs.plugins.kotlin.serialization)
    `maven-publish`
    id("rust-setup")
}

group = "gay.asoji"
version = "${rootProject.property("starmedia_version")}"

base {
    archivesName.set("starmedia")
}

repositories {
    mavenCentral()
}

val rust = natives("starmedia_natives") {
    path = projectDir.toPath().resolve("native")

    platform("windows", "x64")

    // TODO: fix for every other platform
//    platform("windows", "arm64")
//    platform("windows", "arm64ec")

//    platform("linux", "x86_64")
//    platform("linux", "armv7")
//    platform("linux", "armv7s")
//    platform("linux", "arm64")

//    platform("mac", "x86_64")
//    platform("mac", "arm64")
}

dependencies {
    api(libs.bundles.kotlinx.coroutines)

    api(libs.kotlinx.serialization.json)
    api(libs.slf4j.api)

//    api(libs.kache) // not sure if we *need* this but if we do, then yeah

    testRuntimeOnly(libs.slf4j.simple)
    testRuntimeOnly(files(project.tasks.getByName("nativesJarWindowsX64", org.gradle.jvm.tasks.Jar::class)))
}

tasks.test {
    useJUnitPlatform()
}

java {
    withSourcesJar()
}

kotlin {
    jvmToolchain(21)
}

publishing {
    repositories {
        maven("https://mvn.devos.one/snapshots") {
            credentials {
                username = System.getenv()["MAVEN_USER"]
                password = System.getenv()["MAVEN_PASS"]
            }
        }
    }

    publications {
        register("maven", MavenPublication::class) {
            version = "${rootProject.version}"
            from(components.getByName("java"))

            for (platform in rust.platformTaskNames) {
                artifact(tasks.getByName("nativesJar$platform"))
            }
        }
    }
}
