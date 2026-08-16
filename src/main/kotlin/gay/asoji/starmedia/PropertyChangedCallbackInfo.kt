package gay.asoji.starmedia

data class PropertyChangedCallbackInfo(
    val index: Long,
    val receiver: Long,
) {
    companion object {
        fun fromGSMTCS(array: LongArray): PropertyChangedCallbackInfo {
            return PropertyChangedCallbackInfo(array[0], array[1])
        }
    }
}
