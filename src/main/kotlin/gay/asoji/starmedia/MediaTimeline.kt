package gay.asoji.starmedia

data class MediaTimeline(
    val startTime: Long,
    val endTime: Long,
    val position: Long,
    val lastUpdated: Long,
    val minSeekTime: Long,
    val maxSeekTime: Long,
) {
    companion object {
        fun fromGSMTCS(array: LongArray): MediaTimeline {
            return MediaTimeline(
                array[0].windowsTicksToMillis,
                array[1].windowsTicksToMillis,
                array[2].windowsTicksToMillis,
                array[3].windowsTicksToMillis,
                array[5].windowsTicksToMillis,
                array[4].windowsTicksToMillis,
            )
        }

        private inline val Long.windowsTicksToMillis: Long
            get() {
                return this / 10_000
            }
    }
}
