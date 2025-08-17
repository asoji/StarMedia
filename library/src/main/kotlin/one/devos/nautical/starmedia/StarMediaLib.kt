package one.devos.nautical.starmedia

class StarMediaLib {
    external fun requestManager(): Long
    external fun tryPause(gsmtcs: Long)
    external fun metadata(gsmtcs: Long): Array<String?>?
    external fun setPropertyChangedCallback(obj: Any?, gsmtcs: Long)

    init {
        System.loadLibrary("starmedia_natives")
    }
}