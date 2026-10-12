package io.github.kimzam30.nhakobeam

import io.github.kimzam30.nhakobeam.model.FileItem
import io.github.kimzam30.nhakobeam.ui.Words
import io.github.kimzam30.nhakobeam.ui.components.wordsFromCodeUri
import io.github.kimzam30.nhakobeam.ui.formatBytes
import io.github.kimzam30.nhakobeam.ui.formatEta
import io.github.kimzam30.nhakobeam.ui.formatSpeed
import io.github.kimzam30.nhakobeam.ui.middleTruncate
import io.github.kimzam30.nhakobeam.ui.percent
import io.github.kimzam30.nhakobeam.ui.summarizeFiles
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/** The same cases as desktop/src/lib/format.test.ts, so both apps say things the same way. */
class FormatTest {
    @Test fun formatsBytes() {
        assertEquals("0 B", formatBytes(0))
        assertEquals("999 B", formatBytes(999))
        assertEquals("1 KB", formatBytes(1_000))
        assertEquals("1.3 MB", formatBytes(1_250_000))
        assertEquals("1.4 GB", formatBytes(1_400_000_000))
        assertEquals("240 MB", formatBytes(240_000_000))
        assertEquals("48 MB/s", formatSpeed(48_000_000.0))
    }

    @Test fun formatsTimeLeft() {
        assertNull(formatEta(100, 0.0))
        assertNull(formatEta(0, 100.0))
        assertEquals("5 sec", formatEta(5, 1.0))
        assertEquals("5 sec", formatEta(2, 1.0))
        assertEquals("45 sec", formatEta(42, 1.0))
        assertEquals("2 min", formatEta(130, 1.0))
        assertEquals("2 hr 15 min", formatEta(3600 * 2 + 60 * 15, 1.0))
    }

    @Test fun floorsAndClampsPercent() {
        assertEquals(99, percent(999, 1000))
        assertEquals(100, percent(2000, 1000))
        assertEquals(0, percent(1, 0))
    }

    @Test fun summarizesFiles() {
        val img = FileItem("1", "a.jpg", 1, "image/jpeg")
        val vid = FileItem("2", "b.mp4", 1, "video/mp4")
        val doc = FileItem("3", "c.pdf", 1, "application/pdf")
        assertEquals("a.jpg", summarizeFiles(listOf(img)))
        assertEquals("3 photos and 1 video", summarizeFiles(listOf(img, img, img, vid)))
        assertEquals("3 files", summarizeFiles(listOf(img, vid, doc)))
    }

    @Test fun truncatesInTheMiddle() {
        val t = middleTruncate("holiday-video-from-langkawi-final-final.mp4", 24)
        assertTrue(t.endsWith(".mp4"))
        assertTrue(t.length <= 24)
        assertTrue(t.contains('…'))
        assertEquals("a.txt", middleTruncate("a.txt"))
    }

    @Test fun knowsTheCodeWords() {
        assertEquals(256, Words.all.size)
        assertEquals(256, Words.all.toSet().size)
        assertTrue(Words.isWord("River"))
        assertFalse(Words.isWord("notaword"))
        assertTrue("lantern" in Words.complete("lan"))
        assertEquals(listOf("river", "lantern", "seven"), wordsFromCodeUri("nhakobeam://pair/river-lantern-seven"))
        assertNull(wordsFromCodeUri("https://example.com"))
    }
}
