// Sizes, speeds, times and file summaries. The same rules as desktop/src/lib/format.ts
// (docs/design/ux-copy.md): never more than one decimal, "About 2 min left", sentence case.
package io.github.kimzam30.nhakobeam.ui

import io.github.kimzam30.nhakobeam.model.FileItem
import java.util.Locale
import kotlin.math.ceil
import kotlin.math.floor
import kotlin.math.roundToLong

private val UNITS = listOf("B", "KB", "MB", "GB", "TB")

fun formatBytes(n: Long): String = formatBytes(n.toDouble())

fun formatBytes(n: Double): String {
    if (!n.isFinite() || n <= 0) return "0 B"
    var i = 0
    var v = n
    while (v >= 1000 && i < UNITS.lastIndex) {
        v /= 1000
        i++
    }
    val digits = if (i == 0 || v >= 100) 0 else 1
    val text = String.format(Locale.ROOT, "%.${digits}f", v).removeSuffix(".0")
    return "$text ${UNITS[i]}"
}

fun formatSpeed(bytesPerSecond: Double) = "${formatBytes(bytesPerSecond)}/s"

/** "2 min" style, for "About {time} left". Null when there isn't enough to go on. */
fun formatEta(remainingBytes: Long, speed: Double): String? {
    if (speed <= 0 || remainingBytes <= 0) return null
    val s = remainingBytes / speed
    return when {
        s < 60 -> "${(ceil(s / 5) * 5).toLong()} sec"
        s < 3600 -> "${(s / 60).roundToLong()} min"
        else -> {
            val h = floor(s / 3600).toLong()
            val m = ((s % 3600) / 60).roundToLong()
            if (m > 0) "$h hr $m min" else "$h hr"
        }
    }
}

fun percent(done: Long, total: Long): Int = if (total <= 0) 0 else minOf(100, floor(done.toDouble() / total * 100).toInt())

private enum class Kind(val one: String, val many: String) {
    Photo("photo", "photos"), Video("video", "videos"), Song("song", "songs"),
    Document("document", "documents"), Archive("archive", "archives"), File("file", "files"),
}

private val archiveName = Regex("""\.(zip|rar|7z|tar|gz)$""", RegexOption.IGNORE_CASE)

private fun kind(f: FileItem): Kind {
    val m = f.mime
    return when {
        m.startsWith("image/") -> Kind.Photo
        m.startsWith("video/") -> Kind.Video
        m.startsWith("audio/") -> Kind.Song
        Regex("zip|rar|7z|tar|gzip").containsMatchIn(m) || archiveName.containsMatchIn(f.name) -> Kind.Archive
        m.startsWith("text/") || Regex("pdf|document|sheet|presentation|msword").containsMatchIn(m) -> Kind.Document
        else -> Kind.File
    }
}

/** For picking an icon: "photo", "video", "song", "document", "archive" or "file". */
fun fileKindName(f: FileItem) = kind(f).one

private fun count(n: Int, k: Kind) = "$n ${if (n == 1) k.one else k.many}"

/** "3 photos and 1 video", "12 files", or the file name for a single file. */
fun summarizeFiles(files: List<FileItem>): String {
    if (files.isEmpty()) return "No files"
    if (files.size == 1) return files[0].name
    val groups = files.groupingBy { kind(it) }.eachCount()
    if (groups.size > 2) return count(files.size, Kind.File)
    return groups.entries.sortedByDescending { it.value }.joinToString(" and ") { count(it.value, it.key) }
}

/** Truncates in the middle so the extension stays visible: "holiday-vid…final.mp4". */
fun middleTruncate(name: String, max: Int = 28): String {
    if (name.length <= max) return name
    val dot = name.lastIndexOf('.')
    val ext = if (dot > 0 && name.length - dot <= 6) name.substring(dot) else ""
    val keep = max - ext.length - 1
    val head = ceil(keep * 0.6).toInt()
    val tail = keep - head
    val base = if (ext.isNotEmpty()) name.substring(0, dot) else name
    return base.take(head) + "…" + base.takeLast(tail) + ext
}

/** How long something took: "under a second", "12 sec", "4 min 5 sec", "1 hr 20 min". */
fun formatDuration(ms: Long): String {
    val s = Math.round(ms / 1000.0)
    if (s < 1) return "under a second"
    if (s < 60) return "$s sec"
    if (s < 3600) {
        val m = s / 60
        val rest = s % 60
        return if (rest > 0) "$m min $rest sec" else "$m min"
    }
    val h = s / 3600
    val m = Math.round((s % 3600) / 60.0)
    return if (m > 0) "$h hr $m min" else "$h hr"
}

/** Local midnight before [at]: the key History groups a day's entries under. */
fun dayStart(at: Long): Long = java.util.Calendar.getInstance().apply {
    timeInMillis = at
    set(java.util.Calendar.HOUR_OF_DAY, 0)
    set(java.util.Calendar.MINUTE, 0)
    set(java.util.Calendar.SECOND, 0)
    set(java.util.Calendar.MILLISECOND, 0)
}.timeInMillis

/** "Today", "Yesterday", or a date like "Fri, Oct 9" ("Thu, Oct 9, 2025" in another year), in the phone's own style. */
fun dayLabel(at: Long, now: Long, today: String, yesterday: String): String {
    val day = dayStart(at)
    val start = dayStart(now)
    if (day == start) return today
    if (day == dayStart(start - 1)) return yesterday
    val sameYear = java.util.Calendar.getInstance().apply { timeInMillis = at }.get(java.util.Calendar.YEAR) ==
        java.util.Calendar.getInstance().apply { timeInMillis = now }.get(java.util.Calendar.YEAR)
    val pattern = android.text.format.DateFormat.getBestDateTimePattern(java.util.Locale.getDefault(), if (sameYear) "EEEMMMd" else "EEEMMMdyyyy")
    return java.text.SimpleDateFormat(pattern, java.util.Locale.getDefault()).format(java.util.Date(at))
}

/** The time of day, 12 or 24 hour as the phone is set. */
fun timeOfDay(at: Long, is24Hour: Boolean): String =
    java.text.SimpleDateFormat(if (is24Hour) "HH:mm" else "h:mm a", java.util.Locale.getDefault()).format(java.util.Date(at))
