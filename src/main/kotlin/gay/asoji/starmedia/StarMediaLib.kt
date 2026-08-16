package gay.asoji.starmedia

import org.slf4j.Logger
import org.slf4j.LoggerFactory
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.StandardCopyOption
import java.nio.file.StandardOpenOption
import java.security.DigestInputStream
import java.security.MessageDigest
import kotlin.io.path.*

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
    private external fun getStatus(manager: Long): Int

    private external fun dropReceiver(ptr: Long, index: Long)
    private external fun getSongInfo(ptr: Long): Array<Any?>

    fun getStatus(): PlaybackStatus {
        return PlaybackStatus.fromGSMTCS(this.getStatus(this.requestManager()))
    }

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

        val osName = System.getProperty("os.name").lowercase()
        val isWindows = osName.contains("win")
        val isMac = osName.contains("mac")

        val osArch = System.getProperty("os.arch").lowercase().run {
            if (isWindows && this == "amd64")
                "x64"
            else this
        }

        val dir = "starmedia_natives/${if (isWindows) "windows" else if (isMac) "osx" else "linux"}/${osArch}"

        val libraryResource = classLoader.getResource("$dir/$fullLibName")
            ?: throw Exception("Could not locate StarMedia natives for platform $osName $osArch!")

        val tmpDir = Path(System.getProperty("java.io.tmpdir")).resolve("starmedia-natives")

        if (!tmpDir.exists())
            tmpDir.createDirectories()

        // first, try to hash the file
        val digest = MessageDigest.getInstance("MD5")
        libraryResource.openStream().use {
            DigestInputStream(it, digest).readAllBytes()
        }

        val expectedHash = digest.digest().toHexString()
        val hashedTmpDir = tmpDir.resolve(expectedHash)
        if (!hashedTmpDir.exists())
            hashedTmpDir.createDirectories()

        val starMediaPath = hashedTmpDir.resolve(fullLibName)
        val lockFile = hashedTmpDir.resolve("session.lock")

        if (starMediaPath.exists()) {
            // hash the file and see if it matches
            starMediaPath.inputStream(StandardOpenOption.READ).use {
                DigestInputStream(it, digest).readAllBytes()
            }

            // hash matches, we're good
            if (expectedHash == digest.digest().toHexString())
                return hashedTmpDir

            // nope, let's see if someone's currently copying it.
            // if so, we should block the thread until the lock is invalid.
            if (lockFile.exists()) {
                val pid = lockFile.readText().trim().toLongOrNull()
                if (pid != null && ProcessHandle.of(pid).isPresent) {
                    while (true) {
                        // check if the lock file still exists
                        val pid = if (lockFile.exists()) {
                            lockFile.readText().trim().toLongOrNull()
                        } else break

                        // also check if the process still exists
                        if (pid == null || ProcessHandle.of(pid).isEmpty)
                            break

                        // let's not check too frequently...
                        Thread.sleep(2_500L)
                    }

                    // okay, let's check again just to be safe.
                    starMediaPath.inputStream(StandardOpenOption.READ).use {
                        DigestInputStream(it, digest).readAllBytes()
                    }

                    if (expectedHash == digest.digest().toHexString())
                        return hashedTmpDir
                }

                // fuck, okay let's continue extracting us I guess, we assume the program crashed or failed or something.
            }
        }

        libraryResource.openStream().use {
            try {
                // we want to make sure we're not copying all at once.
                lockFile.writeText("${ProcessHandle.current().pid()}", options = arrayOf(StandardOpenOption.WRITE, StandardOpenOption.CREATE, StandardOpenOption.TRUNCATE_EXISTING))

                Files.copy(it, starMediaPath, StandardCopyOption.REPLACE_EXISTING)
            } catch (e: AccessDeniedException) {
                if (!starMediaPath.exists())
                    throw e
            } finally {
                // okay, we're done here.
                lockFile.deleteIfExists()
            }
        }

        if (!starMediaPath.exists())
            throw Exception("Failed to extract library files for StarMedia!")
        else {
            starMediaPath.inputStream(StandardOpenOption.READ).use {
                DigestInputStream(it, digest).readAllBytes()
            }

            // uh oh
            val actualHash = digest.digest().toHexString()
            if (expectedHash != actualHash)
                throw IllegalStateException("Extracted library hash for StarMedia does not match! (expected: $expectedHash, got: $actualHash)")
        }

        return hashedTmpDir
    }

    @JvmStatic
    fun isAvailable(): Boolean {
        this.autoLoad()
        return this.isLoaded
    }
}
