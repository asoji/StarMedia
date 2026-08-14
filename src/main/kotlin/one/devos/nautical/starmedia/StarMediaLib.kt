package one.devos.nautical.starmedia

import org.slf4j.Logger
import org.slf4j.LoggerFactory
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.StandardCopyOption
import kotlin.io.path.Path
import kotlin.io.path.absolutePathString
import kotlin.io.path.createDirectories
import kotlin.io.path.exists

object StarMediaLib {
    private external fun requestManager(): Long
    private external fun tryPlay(manager: Long): Boolean
    private external fun tryPause(manager: Long): Boolean
    private external fun tryTogglePlayPause(manager: Long): Boolean
    private external fun trySkipNext(manager: Long): Boolean
    private external fun trySkipPrevious(manager: Long): Boolean
    private external fun metadata(manager: Long): Array<Any?>?
    private external fun setPropertyChangedCallback(manager: Long): LongArray
    private external fun timeline(manager: Long): LongArray

    private external fun dropReceiver(ptr: Long, index: Long)
    private external fun getSongInfo(ptr: Long): Array<Any?>

    fun tryPlay(): Boolean = this.tryPlay(this.requestManager())
    fun tryPause(): Boolean = this.tryPause(this.requestManager())
    fun tryTogglePlayPause(): Boolean = this.tryTogglePlayPause(this.requestManager())
    fun trySkipNext(): Boolean = this.trySkipNext(this.requestManager())
    fun trySkipPrevious(): Boolean = this.trySkipPrevious(this.requestManager())
    fun metadata(): MediaMetadata? = this.metadata(this.requestManager())
        ?.let { MediaMetadata.fromGSMTCS(it) }
    fun timeline(): MediaTimeline? = try {
        MediaTimeline.fromGSMTCS(this.timeline(this.requestManager()))
    } catch (_: Throwable) { null }

    fun addPropertyChangedCallback(): PropertyChangedCallbackInfo = this.setPropertyChangedCallback(this.requestManager())
        .let { PropertyChangedCallbackInfo.fromGSMTCS(it) }

    // Taken from UnityTranslateLib [thanks Naz]
    val logger: Logger = LoggerFactory.getLogger(StarMediaLib::class.java)
    private var isLoaded = false

    @JvmStatic
    fun autoLoad() {
        try {
            this.autoLoadOrThrow()
        } catch (e: Throwable) {
            logger.error("Failed to load StarMedia!", e)
            logger.warn("StarMedia may not be supported on platform ${System.getProperty("os.name")} (${System.getProperty("os.arch")})!")
            logger.warn("As a result, StarMedia will not be running, and may cause errors if any native calls are attempted.")
        }
    }

    // Modified from ImGui-java's library loading - https://github.com/SpaiR/imgui-java/blob/main/imgui-binding/src/main/java/imgui/ImGui.java
    @JvmStatic
    fun autoLoadOrThrow() {
        if (this.isLoaded)
            return

        val libPath = System.getProperty("starmedia.library.path")
        val fullLibName = System.getProperty("starmedia.library.name", System.mapLibraryName("starmedia_natives"))

        if (libPath != null) {
            System.load(Path(libPath).resolve(fullLibName).absolutePathString())
        } else {
            try {
                System.loadLibrary(fullLibName)
            } catch (e: Throwable) {
                val extractedPath = try {
                    tryLoadFromClassPath(fullLibName)
                } catch (e2: Exception) {
                    val joined = RuntimeException("Failed to load natives for StarMedia!")
                    joined.addSuppressed(e2)
                    joined.addSuppressed(e)

                    throw joined
                }

                System.load(extractedPath.resolve(fullLibName).absolutePathString())
            }
        }

        this.isLoaded = true
    }

    private fun tryLoadFromClassPath(fullLibName: String): Path {
        val classLoader = StarMediaLib::class.java.classLoader

        val tmpDir = Path(System.getProperty("java.io.tmpdir")).resolve("starmedia-natives")

        if (!tmpDir.exists())
            tmpDir.createDirectories()

        val osName = System.getProperty("os.name").lowercase()
        val isWindows = osName.contains("win")
        val isMac = osName.contains("mac")

        val osArch = System.getProperty("os.arch").lowercase().run {
            if (isWindows && this == "amd64")
                "x64"
            else this
        }

        val dir = "starmedia_natives/${if (isWindows) "windows" else if (isMac) "osx" else "linux"}/${osArch}"

        classLoader.getResourceAsStream("$dir/$fullLibName")?.use {
            val libPath = tmpDir.resolve(fullLibName)
            try {
                Files.copy(it, libPath, StandardCopyOption.REPLACE_EXISTING)
            } catch (e: AccessDeniedException) {
                if (!libPath.exists())
                    throw e
            }
        }

        val starMediaPath = tmpDir.resolve(fullLibName)
        if (!starMediaPath.exists())
            throw Exception("Failed to extract library files for StarMedia!")

        return tmpDir
    }

    @JvmStatic
    fun isAvailable(): Boolean {
        this.autoLoad()
        return this.isLoaded
    }
}
