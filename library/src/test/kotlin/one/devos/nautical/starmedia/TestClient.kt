package one.devos.nautical.starmedia

import java.lang.Thread.sleep

fun main(args: Array<String>) {
    while (true) {
        // request last known session id from the session manager
        val gsmtc = StarMediaLib().requestManager()

        // grab metadata from last known session id
        val metadata = StarMediaLib().metadata(gsmtc)

        sleep(1000)

        println("GSMTC Long ID: $gsmtc")
        // print out the metadata array
        // uh oh why is it not going through UTF-8? uhhh
//        println(metadata?.joinToString("\n")) [this works but im just commenting it out bc of testing below]


        println(metadata?.get(0)) // title?
        println(metadata?.get(1)) // this should be artist but uh
        println(metadata?.get(2)) // subtitle?
        println(metadata?.get(3)) // album name [doesnt seem to be passed through by apple music at least] =/
        println(metadata?.get(4)) // album artist?
        println(metadata?.get(5)) // album length [just gives 0 rn?
        println(metadata?.get(6)) // track number?

        // tries to pause music every second bc of the sleep lmao
//        StarMediaLib().tryPause(gsmtc)
    }
}