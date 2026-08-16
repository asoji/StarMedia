package gay.asoji.starmedia

enum class PlaybackStatus {
    CHANGING, STOPPED,
    PLAYING, PAUSED,

    UNKNOWN,;

    companion object {
        fun fromGSMTCS(value: Int): PlaybackStatus {
            return when (value) {
                2 -> CHANGING
                3 -> STOPPED
                4 -> PLAYING
                5 -> PAUSED

                else -> UNKNOWN
            }
        }
    }
}