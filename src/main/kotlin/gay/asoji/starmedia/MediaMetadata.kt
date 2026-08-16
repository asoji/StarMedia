package gay.asoji.starmedia

import java.nio.ByteBuffer

data class MediaMetadata(
    val title: String,
    val artist: String,
    val subtitle: String,
    val albumName: String,
    val albumArtist: String,
    val albumTrackCount: Int,
    val trackNumber: Int,
    val icon: ByteBuffer,
) {
    companion object {
        fun fromGSMTCS(props: Array<Any?>): MediaMetadata {
            return MediaMetadata(
                props[0] as String,
                props[1] as String,
                props[2] as String,
                props[3] as String,
                props[4] as String,
                props[5] as Int,
                props[6] as Int,
                props[7] as ByteBuffer,
            )
        }
    }
}
