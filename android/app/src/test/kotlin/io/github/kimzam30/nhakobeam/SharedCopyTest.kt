package io.github.kimzam30.nhakobeam

import io.github.kimzam30.nhakobeam.ui.Copy
import java.io.File
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/** The Android strings are the desktop's (desktop/src/lib/copy.ts), and every key the code asks for exists. */
class SharedCopyTest {
    private val repo = File("../..").canonicalFile

    /** 'key': 'value' or "value", with \' and \" escapes, as copy.ts writes them. */
    private fun desktopStrings(): Map<String, String> {
        val text = File(repo, "desktop/src/lib/copy.ts").readText()
        val entry = Regex("""^\s*'([\w.]+)':\s*(?:'((?:[^'\\]|\\.)*)'|"((?:[^"\\]|\\.)*)"),?\s*$""", RegexOption.MULTILINE)
        return entry.findAll(text).associate { m ->
            val raw = m.groupValues[2].ifEmpty { m.groupValues[3] }
            m.groupValues[1] to raw.replace("\\'", "'").replace("\\\"", "\"").replace("\\\\", "\\")
        }
    }

    @Test fun matchesTheDesktopApp() {
        val desktop = desktopStrings()
        assertTrue("copy.ts parsed", desktop.size > 200)
        assertEquals("Copy.kt is out of date: run node android/tools/shared-assets.mjs", desktop, Copy.strings)
    }

    @Test fun everyKeyTheCodeUsesExists() {
        val call = Regex("""\bt\("([\w.]+)"""")
        val missing = File(repo, "android/app/src/main/kotlin").walk().filter { it.extension == "kt" }
            .flatMap { f -> call.findAll(f.readText()).map { f.name to it.groupValues[1] } }
            .filter { (_, key) -> key !in Copy.strings }
            .toList()
        assertEquals("unknown copy keys", emptyList<Pair<String, String>>(), missing)
    }

    @Test fun fillsPlaceholders() {
        assertEquals("Sending to Studio PC", Copy.t("transfer.sending", "device" to "Studio PC"))
    }
}
