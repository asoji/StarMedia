package one.devos.nautical.starmedia

import org.slf4j.Logger
import org.slf4j.LoggerFactory
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import java.nio.file.StandardCopyOption
import kotlin.io.path.absolutePathString
import kotlin.io.path.exists

class StarMediaLib {
    external fun requestManager(): Long
    external fun tryPause(gsmtcs: Long)
    external fun metadata(gsmtcs: Long): Array<String?>?
    external fun setPropertyChangedCallback(obj: Any?, gsmtcs: Long)

    // Taken from UnityTranslateLib [thanks Naz]
    companion object {
        val logger: Logger = LoggerFactory.getLogger(StarMediaLib::class.java)
        private val platformLibs: List<String>
            get() {
                val osName = System.getProperty("os.name").lowercase()
                val osArch = System.getProperty("os.arch").lowercase()
                val isWindows = osName.contains("win")
                val isMac = osName.contains("mac")

                val dir = "starmedia/${if (isWindows) "windows" else if (isMac) "osx" else "linux"}-${osArch}"

                return if (osArch == "amd64") {
                    if (isWindows)
                        listOf(
                            "$dir/starmedia_natives.dll"
                        ) else emptyList()
                } else emptyList()
            }
        private val cachedPlatformLibs = platformLibs

        // Modified from ImGui-java's library loading - https://github.com/SpaiR/imgui-java/blob/main/imgui-binding/src/main/java/imgui/ImGui.java
        init {
            if (isAvailable()) {
                val libPath = System.getProperty("starmedia.library.path")
                val libName = System.getProperty("starmedia.library.name", "starmedia_natives")
                val fullLibName = resolveFullLibName()

                if (libPath != null) {
                    System.load(Paths.get(libPath).resolve(fullLibName).absolutePathString())
                } else {
                    try {
                        System.loadLibrary(libName)
                    } catch (e: Throwable) {
                        val extractedPath = try {
                            tryLoadFromClassPath(fullLibName)
                        } catch (e2: Exception) {
                            val joined = RuntimeException("Failed to load natives for StarMedia!")
                            joined.addSuppressed(e2)
                            joined.addSuppressed(e)

                            throw joined
                        }

                        val osName = System.getProperty("os.name").lowercase()
                        val osArch = System.getProperty("os.arch").lowercase()
                        val isWindows = osName.contains("win")
                        val isMac = osName.contains("mac")
                        val dir =
                            "starmedia/${if (isWindows) "windows" else if (isMac) "osx" else "linux"}-${osArch}/"

                        for (lib in platformLibs.reversed()) {
                            System.load(extractedPath.resolve(lib.removePrefix(dir)).absolutePathString())
                        }
                    }
                }
            }
        }

        private fun resolveFullLibName(): String {
            val osName = System.getProperty("os.name").lowercase()
            val isWindows = osName.contains("win")
            val isMac = osName.contains("mac")

            val libPrefix = if (isWindows) "" else "lib"
            val libSuffix = if (isWindows) ".dll" else if (isMac) ".dylib" else ".so"

            return System.getProperty("starmedia.library.name", "${libPrefix}starmedia_natives${libSuffix}")
        }

        private fun tryLoadFromClassPath(fullLibName: String): Path {
            val classLoader = StarMediaLib::class.java.classLoader
            val libs = platformLibs

            if (libs.isEmpty())
                throw Exception("Unsupported platform ${System.getProperty("os.name")} (${System.getProperty("os.arch")})!")

            val tmpDir = Paths.get(System.getProperty("java.io.tmpdir")).resolve("starmedia-natives")

            if (!tmpDir.exists())
                tmpDir.toFile().mkdirs()

            for (packedLibPath in libs) {
                val libName = packedLibPath.split("/").last()

                classLoader.getResourceAsStream(packedLibPath)?.use {
                    val libPath = tmpDir.resolve(libName)
                    try {
                        Files.copy(it, libPath, StandardCopyOption.REPLACE_EXISTING)
                    } catch (e: AccessDeniedException) {
                        if (!libPath.exists())
                            throw e
                    }
                }
            }

            val unityTranslatePath = tmpDir.resolve(fullLibName)
            if (!unityTranslatePath.exists())
                throw Exception("Failed to load library files for StarMedia!")

            return tmpDir
        }

        fun isAvailable(): Boolean {
            return cachedPlatformLibs.isNotEmpty()
        }
    }
}