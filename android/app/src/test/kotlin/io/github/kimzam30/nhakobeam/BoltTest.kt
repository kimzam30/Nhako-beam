package io.github.kimzam30.nhakobeam

import androidx.compose.ui.geometry.Offset
import io.github.kimzam30.nhakobeam.ui.components.BOLT_POINTS
import io.github.kimzam30.nhakobeam.ui.components.CHECK_POINTS
import io.github.kimzam30.nhakobeam.ui.components.arcPoints
import io.github.kimzam30.nhakobeam.ui.components.boltCorners
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/** The bolt's geometry: the same numbers as desktop/src/lib/motion/bolt.ts (bolt.test.ts). */
class BoltTest {
    @Test fun morphRunsFromTheBoltToTheCheck() {
        assertEquals(BOLT_POINTS, boltCorners(0f))
        assertEquals(CHECK_POINTS, boltCorners(1f))
        // Half way is half way, corner by corner, and it never runs past either end.
        val (x, y) = boltCorners(0.5f)[4]
        assertEquals((194.24f + 166f) / 2, x, 0.01f)
        assertEquals((862.4f + 544f) / 2, y, 0.01f)
        assertEquals(CHECK_POINTS, boltCorners(3f))
    }

    @Test fun anArcStartsAndEndsWhereItShouldAndZigs() {
        val a = Offset(10f, 40f)
        val b = Offset(200f, 40f)
        val pts = arcPoints(a, b, seed = 1, amp = 7f)
        assertEquals(a, pts.first())
        assertEquals(b, pts.last())
        assertTrue(pts.size in 5..10)
        // The corners in between leave the straight line, on both sides.
        val off = pts.drop(1).dropLast(1).map { it.y - 40f }
        assertTrue(off.any { it > 1f } && off.any { it < -1f })
        assertTrue(off.all { kotlin.math.abs(it) <= 7f })
    }
}
