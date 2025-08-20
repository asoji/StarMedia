plugins {
    alias(libs.plugins.kotlin)
    alias(libs.plugins.kotlin.serialization)
}

repositories {
    mavenCentral()
}

dependencies {
    testImplementation(kotlin("test"))

    api(libs.bundles.kotlinx.coroutines)

    api(libs.kotlinx.serialization.json)
    api(libs.slf4j.api)

//    api(libs.kache) // not sure if we *need* this but if we do, then yeah

    testRuntimeOnly(libs.slf4j.simple)
}

publishing {
    publications {
        register("maven", MavenPublication::class) {
            groupId = "one.devos.nautical"
            artifactId = "starmedia"
            version = "${rootProject.property("starmedia_version")}"
            from(components.getByName("java"))
        }
    }
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