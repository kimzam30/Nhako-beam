package io.github.kimzam30.nhakobeam.stage

import io.github.kimzam30.nhakobeam.model.Device
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import kotlin.math.abs

// Home's room, as data (design/room/build.mjs writes room.json next to the images). Where things go,
// and which device stands where. The desktop twin is desktop/src/lib/stage/room.ts.

@Serializable data class RoomRect(val x: Float, val y: Float, val w: Float, val h: Float = w)
@Serializable data class RoomBox(val x0: Float, val x1: Float, val y0: Float, val y1: Float)
@Serializable data class RoomStanding(val x: Float, val y: Float, val box: RoomBox, val art: RoomRect, val margin: Float)
@Serializable data class RoomSlot(val id: String, val on: String, val z: Float, val kinds: Map<String, RoomStanding>)
@Serializable data class RoomPoint(val x: Float, val y: Float)
@Serializable data class RoomBolt(val x: Float, val y: Float, val size: Float, val threadX: Float, val threadY: Float)
@Serializable data class RoomBand(val top: Float, val bottom: Float)
@Serializable data class RoomSides(val left: Float, val right: Float)
@Serializable data class RoomSize(val w: Float, val h: Float)
@Serializable data class RoomBoltLit(val x: Float, val y: Float, val w: Float, val unit: Float)
@Serializable data class RoomFitRules(val tallBelow: Float, val midBelow: Float, val band: RoomBand, val wideFocus: RoomSides, val coverBelow: RoomSize)
@Serializable data class RoomCovers(val top: Float, val bottom: Float)
@Serializable data class RoomChrome(val shortBelow: Float, val narrowBelow: Float, val short: RoomCovers, val narrow: RoomCovers, val wide: RoomCovers, val panelW: Float)
@Serializable data class RoomTagSize(val w: Float, val h: Float, val gap: Float)

@Serializable
data class RoomData(
    val views: Map<String, RoomRect>,
    val person: RoomRect,
    val bolt: RoomBolt,
    val boltLit: RoomBoltLit,
    val front: RoomRect,
    val fit: RoomFitRules,
    val chrome: RoomChrome,
    val tag: RoomTagSize,
    val steam: RoomPoint,
    val z: RoomPoint,
    val cone: List<List<Float>>,
    val window: RoomRect,
    val deviceArt: Map<String, Int>,
    val slots: List<RoomSlot>,
) {
    companion object {
        private val json = Json { ignoreUnknownKeys = true }
        fun parse(text: String): RoomData = json.decodeFromString(serializer(), text)
    }
}

/** A device at its place. */
data class RoomSpot(val device: Device, val slot: String, val on: String, val z: Float, val at: RoomStanding)

/** Who's in the room, and everyone else (the bubbles under the buttons). */
data class RoomLayout(val room: List<RoomSpot>, val rest: List<Device>)

/** The room on a screen, in screen points. */
data class RoomFit(val view: String, val k: Float, val ox: Float, val oy: Float, val top: Float, val bottom: Float, val panelW: Float)

/** A name tag's place on screen, in screen points. */
data class RoomTagAt(val id: String, val x: Float, val y: Float, val w: Float)

/** Trusted first, then by name, and never by whether it's online: nothing moves when a device sleeps. */
fun roomOrder(devices: List<Device>): List<Device> = io.github.kimzam30.nhakobeam.state.withoutTwins(devices).sortedWith(compareBy({ !it.trusted }, { it.name.lowercase() }, { it.id }))

/** Which crop of the room a screen uses: rotating a screen just changes crop. */
fun roomView(data: RoomData, width: Float, height: Float): String {
    val a = width / height
    return if (a <= data.fit.tallBelow) "tall" else if (a <= data.fit.midBelow) "mid" else "wide"
}

/** The room fills the box: world units to points. What must stay in view sits between the top buttons and the bottom panel. */
fun fitRoom(data: RoomData, width: Float, height: Float): RoomFit {
    val view = roomView(data, width, height)
    val v = data.views.getValue(view)
    val k = maxOf(width / v.w, height / v.h)
    val c = data.chrome
    val covers = if (height < c.shortBelow) c.short else if (width < c.narrowBelow) c.narrow else c.wide
    val seenW = width / k
    val seenH = height / k
    val f = data.fit.wideFocus
    val left = if (view == "wide") maxOf(v.x, minOf(v.x + v.w - seenW, f.right - seenW, f.left)) else v.x + (v.w - seenW) / 2
    val wanted = (data.fit.band.top + data.fit.band.bottom) / 2 - (covers.top + height - covers.bottom) / 2 / k
    val top = maxOf(v.y, minOf(v.y + v.h - seenH, wanted))
    return RoomFit(view, k, -left * k, -top * k, covers.top, covers.bottom, if (width < c.narrowBelow) width else minOf(c.panelW, width))
}

/** The places this screen offers: a kind stays only if the device and its name tag land on clear screen. */
fun roomSlots(data: RoomData, width: Float, height: Float): List<RoomSlot> {
    if (width < data.fit.coverBelow.w && height < data.fit.coverBelow.h) return emptyList()
    val f = fitRoom(data, width, height)
    val panelL = (width - f.panelW) / 2
    val panelR = (width + f.panelW) / 2
    val panelY = height - f.bottom
    val tagRoom = if (height < data.chrome.shortBelow) 34f else 54f
    val above = 2 * data.tag.h + 8
    return data.slots.mapNotNull { s ->
        val kinds = s.kinds.filterValues { q ->
            val x0 = f.ox + q.box.x0 * f.k
            val x1 = f.ox + q.box.x1 * f.k
            val y0 = f.oy + q.box.y0 * f.k
            val y1 = f.oy + q.box.y1 * f.k + tagRoom
            when {
                x0 < 4 || x1 > width - 4 || y0 - (if (s.on == "hutch") above else 0f) < f.top * 0.75f || y1 > height -> false
                else -> !(y1 > panelY && x1 > panelL && x0 < panelR)
            }
        }
        if (kinds.isEmpty()) null else s.copy(kinds = kinds)
    }
}

/** How many devices this screen's room can hold. */
fun roomMax(data: RoomData, width: Float, height: Float): Int = roomSlots(data, width, height).size

/** Your picks (or, until you've picked, the first devices) get the first free place that holds them. With none left, a device goes in the rest. */
fun arrangeRoom(devices: List<Device>, picks: Set<String>?, data: RoomData, width: Float, height: Float): RoomLayout {
    val free = roomSlots(data, width, height).toMutableList()
    val ordered = roomOrder(devices)
    val picked = (if (picks == null) ordered else ordered.filter { it.id in picks }).take(free.size)
    // Big things choose first: a PC needs a desk, a phone fits nearly anywhere.
    val room = picked.sortedBy { listOf("desktop", "laptop", "tablet", "phone").indexOf(it.kind.name.lowercase()) }.mapNotNull { d ->
        val kind = d.kind.name.lowercase()
        val at = free.indexOfFirst { kind in it.kinds }
        if (at < 0) return@mapNotNull null
        val slot = free.removeAt(at)
        RoomSpot(d, slot.id, slot.on, slot.z, slot.kinds.getValue(kind))
    }
    return RoomLayout(room, ordered.filter { d -> room.none { it.device == d } })
}

/** Each name goes under its device, on the hutch above it so it never covers the desk's monitor. If two would touch, the later moves one row away from its device. A tag is centred in its box of `tag.w`, and only as wide as its name, so `w` is the room it really takes. */
fun roomTags(data: RoomData, room: List<RoomSpot>, f: RoomFit, width: Float): List<RoomTagAt> {
    val w = data.tag.w
    val h = data.tag.h
    val out = mutableListOf<RoomTagAt>()
    for (q in room.sortedBy { it.at.x }) {
        val above = q.on == "hutch"
        val x = (f.ox + q.at.x * f.k - w / 2).coerceIn(6f, maxOf(6f, width - w - 6))
        val wide = (q.device.name.length * 7 + 22f).coerceIn(44f, w)
        var y = if (above) f.oy + q.at.box.y0 * f.k - h - 3 else f.oy + q.at.y * f.k + data.tag.gap
        while (out.any { abs(x - it.x) < (wide + it.w) / 2 + 2 && y < it.y + h + 2 && it.y < y + h + 2 }) y += if (above) -(h + 3) else h + 3
        out += RoomTagAt(q.device.id, x, y, wide)
    }
    return out
}

/** Which picture a device gets at its place: the same wallpaper every time, so a phone keeps its own. */
fun deviceArt(q: RoomSpot, data: RoomData): String {
    val d = q.device
    val kind = d.kind.name.lowercase()
    if (!d.reachable && !d.wakeable) return "device-${q.slot}-$kind-off"
    val count = data.deviceArt.getValue(kind)
    return "device-${q.slot}-$kind-${Math.floorMod(d.id.hashCode(), count)}"
}
