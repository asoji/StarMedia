package gay.asoji.starmedia

import java.io.OutputStreamWriter
import java.io.PrintWriter
import java.lang.Thread.sleep

fun main(args: Array<String>) {
    while (true) {
        // request last known session id from the session manager
        StarMediaLib.autoLoadOrThrow()

        // grab metadata from last known session id
        val metadata = StarMediaLib.metadata()

        sleep(1000)

        val console = PrintWriter(OutputStreamWriter(System.out, Charsets.UTF_8), true)
        console.println(StarMediaLib.timeline())

//        console.println("GSMTC Long ID: $gsmtc")
        // print out the metadata array
        // uh oh why is it not going through UTF-8? uhhh
//        println(metadata?.joinToString("\n")) [this works but im just commenting it out bc of testing below]

        console.println(metadata)
//        console.println(metadata?.get(0)) // title?
//        console.println(metadata?.get(1)) // this should be artist but uh
//        console.println(metadata?.get(2)) // subtitle?
//        console.println(metadata?.get(3)) // album name [doesnt seem to be passed through by apple music at least] =/
//        console.println(metadata?.get(4)) // album artist?
//        console.println(metadata?.get(5)) // album length [just gives 0 rn?
//        console.println(metadata?.get(6)) // track number?

        // tries to pause music every second bc of the sleep lmao
//        StarMediaLib().tryPause(gsmtc)
    }
}