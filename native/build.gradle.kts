data class RustPlatform(
    val targetName: String, val systemName: String, val architecture: String, val exportedFiles: List<String>
) {
    fun isHost(): Boolean {
        val hostOs = System.getProperty("os.name").lowercase()

        // is there seriously no better way to do this
        if ((systemName == "windows" && !hostOs.contains("windows")) || (systemName == "linux" && (!hostOs.contains("linux")))) return false

        return System.getProperty("os.arch") == this.architecture
    }
}

val RUST_TARGETS = listOf(
    RustPlatform("x86_64-pc-windows-msvc", "windows", "amd64", listOf("starmedia_natives.dll")), // Windows x86-64
)

open class ExecutableTask @Inject constructor(@Internal val execOperations: ExecOperations) : DefaultTask()

tasks {
    // TODO: figure out cross-compilation.
    this.register<ExecutableTask>("rustBuild") // TODO: figure out cross-compilation.
    {
        outputs.upToDateWhen { false }

        doFirst {
            for (target in RUST_TARGETS) {
                if (!target.isHost()) // TODO: figure out cross-compilation.
                    continue

                execOperations.exec {
                    commandLine(if (target.isHost()) "cargo" else "cross")

                    val args = mutableListOf(
                        "build", "--release", "--target", target.targetName, "--package", "starmedia_natives", "--lib"
                    )

                    args(args)
                    standardOutput = System.out
                }.assertNormalExitValue()
            }
        }
    }

    register("moveTargetFiles") {
        dependsOn("rustBuild")

        doFirst {
            val nativesDir = layout.buildDirectory.get().dir("starmedia_natives")
            if (!nativesDir.asFile.exists()) nativesDir.asFile.mkdirs()

            val targetsDir = layout.projectDirectory.dir("target")
            for (target in RUST_TARGETS) {
                val starMediaNativesDir = nativesDir.dir("starmedia").dir("${target.systemName}-${target.architecture}")

                if (!target.isHost()) continue

                if (!starMediaNativesDir.asFile.exists()) starMediaNativesDir.asFile.mkdirs()

                val targetDir = targetsDir.dir(target.targetName).dir("release")
                for (fileName in target.exportedFiles) {
                    val srcFile = targetDir.file(fileName).asFile
                    val file = starMediaNativesDir.file(fileName).asFile
                    if (!file.exists()) file.createNewFile()

                    srcFile.copyTo(file, true)
                }
            }
        }
    }

    processResources {
        dependsOn("rustBuild", "moveTargetFiles")

        from(layout.buildDirectory.get().dir("starmedia_natives")).into("natives")
    }

    jar {
        val target = RUST_TARGETS.first { it.isHost() }

        archiveBaseName.set("StarMedia-natives-${target.systemName}-${target.architecture}")
    }
}

publishing {
    val target = RUST_TARGETS.first { it.isHost() }

    publications {
        register("maven", MavenPublication::class) {
            groupId = "one.devos.nautical"
            artifactId = "StarMedia-natives-${target.systemName}-${target.architecture}"
            version = "${rootProject.property("starmedia_version")}"
            from(components.getByName("java"))
        }
    }
}