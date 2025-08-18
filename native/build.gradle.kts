import com.google.gson.JsonParser
import java.net.URI
import java.net.http.HttpClient
import java.net.http.HttpRequest
import java.net.http.HttpResponse
import java.util.zip.ZipFile
import kotlin.time.Duration.Companion.minutes
import kotlin.time.toJavaDuration

val starmediaVersion = rootProject.property("starmedia_version")!! as String

data class RustPlatform(
    val targetName: String,
    val systemName: String,
    val architecture: String,
    val exportedFiles: List<String>
) {
    val starmediaName =
        "${if (systemName == "osx") "macos" else systemName}-${if (architecture == "amd64") "auto64" else if (systemName == "osx") "arm64" else "aarch64"}"

    fun isHost(): Boolean {
        val hostOs = System.getProperty("os.name").lowercase()

        // is there seriously no better way to do this
        if (
            (systemName == "windows" && !hostOs.contains("windows")) ||
            (systemName == "linux" && (!hostOs.contains("linux")))
        )
            return false

        return System.getProperty("os.arch") == this.architecture
    }
}

val RUST_TARGETS = listOf(
    RustPlatform("x86_64-pc-windows-msvc", "windows", "amd64", listOf("starmedia_natives.dll")), // Windows x86-64
)

open class ExecutableTask @Inject constructor(@Internal val execOperations: ExecOperations) : DefaultTask()

data class Platform(
    val distribution: String,
    val architecture: String,
    val outputDist: String,
    val outputArch: String
)

interface FileMatcher {
    val fileName: String

    fun match(text: String): Boolean
}

class StringBased(val text: String, override val fileName: String) : FileMatcher {
    override fun match(text: String): Boolean {
        return this.text == text
    }
}

class RegexBased(val regex: Regex, override val fileName: String) : FileMatcher {
    override fun match(text: String): Boolean {
        return regex.matches(text)
    }
}

tasks {
    create<ExecutableTask>("rustBuild") {
        outputs.upToDateWhen { false }

        doFirst {
            for (target in RUST_TARGETS) {
                if (!target.isHost()) // TODO: figure out cross-compilation.
                    continue

                execOperations.exec {
                    commandLine(if (target.isHost()) "cargo" else "cross")

                    val args = mutableListOf(
                        "build", "--release",
                        "--target", target.targetName,
                        "--package", "starmedia_natives",
                        "--lib"
                    )

                    args(args)
                    standardOutput = System.out
                }
                    .assertNormalExitValue()
            }
        }
    }

//    create("moveTargetFiles") {
//        dependsOn("rustBuild")
//
//        doFirst {
//            val targetsDir = layout.projectDirectory.dir("target")
//            for (target in RUST_TARGETS) {
//                val utNativesDir = nativesDir.dir("unitytranslate").dir("${target.systemName}-${target.architecture}")
//
//                if (!target.isHost())
//                    continue
//
//                if (!utNativesDir.asFile.exists())
//                    utNativesDir.asFile.mkdirs()
//
//                val libDir = starmediaDir.dir(target.starmediaName)
//                val libFile = libDir.asFile
//                val files = libFile.listFiles().map { it.name }
//
//                for (fileName in files) {
//                    val srcFile = libDir.file(fileName).asFile
//                    val file = utNativesDir.file(fileName).asFile
//                    if (!file.exists())
//                        file.createNewFile()
//
//                    srcFile.copyTo(file, true)
//                }
//
//                val targetDir = targetsDir.dir(target.targetName).dir("release")
//                for (fileName in target.exportedFiles) {
//                    val srcFile = targetDir.file(fileName).asFile
//                    val file = utNativesDir.file(fileName).asFile
//                    if (!file.exists())
//                        file.createNewFile()
//
//                    srcFile.copyTo(file, true)
//                }
//            }
//        }
//    }

    processResources {
        dependsOn("rustBuild")

//        from(layout.buildDirectory.get().dir("ut_natives")).into("natives")
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