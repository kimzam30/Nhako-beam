package io.github.kimzam30.nhakobeam.state

import io.github.kimzam30.nhakobeam.model.DiscoveryState
import io.github.kimzam30.nhakobeam.model.Transfer
import io.github.kimzam30.nhakobeam.model.TransferDirection
import io.github.kimzam30.nhakobeam.model.TransferStatus
import io.github.kimzam30.nhakobeam.ui.Copy.t
import io.github.kimzam30.nhakobeam.ui.percent

/** Drives the dot beside Home's heading: steady when ready, pulsing while it looks or works. */
enum class StatusTone { Ready, Searching, Busy, Warn }

/** Home's heading: what the app is doing right now, in one line. The desktop app has the same rules
 *  in lib/state/status.ts; change both together. [key] changes only when the line says something
 *  new (not with every percent), so the heading rolls to a new line once rather than on each tick. */
data class HomeStatus(val key: String, val text: String, val tone: StatusTone, val percent: Int? = null)

/** In order: no network, transfers, files waiting for a device, devices found, looking, ready. */
fun homeStatus(
    discovery: DiscoveryState,
    active: List<Transfer>,
    reachable: Int,
    staging: Boolean,
    deviceName: (String) -> String,
): HomeStatus {
    if (discovery == DiscoveryState.Offline) return HomeStatus("offline", t("home.status.offline"), StatusTone.Warn)
    if (active.size > 1) return HomeStatus("many", t("home.status.many", "n" to active.size), StatusTone.Busy)
    active.firstOrNull()?.let { x ->
        val device = deviceName(x.deviceId)
        val pct = percent(x.doneBytes, x.totalBytes)
        fun at(key: String, text: String, tone: StatusTone, percent: Int? = null) = HomeStatus("$key:${x.id}", text, tone, percent)
        return when (x.status) {
            TransferStatus.Waiting -> at("waiting", t("home.status.waiting", "device" to device), StatusTone.Busy)
            TransferStatus.Connecting, TransferStatus.Reconnecting -> at("connecting", t("home.status.connecting", "device" to device), StatusTone.Busy)
            TransferStatus.Paused -> at("paused", t("home.status.paused", "device" to device), StatusTone.Warn, pct)
            else ->
                if (x.direction == TransferDirection.Send) at("sending", t("home.status.sending", "device" to device), StatusTone.Busy, pct)
                else at("receiving", t("home.status.receiving", "device" to device), StatusTone.Busy, pct)
        }
    }
    if (staging) return HomeStatus("staged", t("home.status.staged"), StatusTone.Ready)
    if (discovery == DiscoveryState.Blocked) return HomeStatus("blocked", t("home.status.blocked"), StatusTone.Warn)
    if (reachable > 0) {
        return HomeStatus(
            "nearby",
            if (reachable == 1) t("home.status.nearby.one") else t("home.status.nearby.many", "n" to reachable),
            StatusTone.Ready,
        )
    }
    if (discovery == DiscoveryState.Searching) return HomeStatus("searching", t("home.searching"), StatusTone.Searching)
    return HomeStatus("ready", t("home.title"), StatusTone.Ready)
}
