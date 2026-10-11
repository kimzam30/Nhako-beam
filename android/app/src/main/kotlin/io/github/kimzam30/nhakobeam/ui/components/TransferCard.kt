// A transfer: who, what, the beam, the numbers, and what you can do (pause, cancel, open).
// The Compose twin of desktop/src/components/TransferCard.svelte and Beam.svelte.
package io.github.kimzam30.nhakobeam.ui.components

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.semantics.ProgressBarRangeInfo
import androidx.compose.ui.semantics.progressBarRangeInfo
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import io.github.kimzam30.nhakobeam.design.NhakoMotion
import io.github.kimzam30.nhakobeam.design.NhakoSpace
import io.github.kimzam30.nhakobeam.design.NhakoType
import io.github.kimzam30.nhakobeam.model.Route
import io.github.kimzam30.nhakobeam.model.Side
import io.github.kimzam30.nhakobeam.model.Transfer
import io.github.kimzam30.nhakobeam.model.TransferDirection
import io.github.kimzam30.nhakobeam.model.TransferError
import io.github.kimzam30.nhakobeam.model.TransferStatus
import io.github.kimzam30.nhakobeam.state.AppState
import io.github.kimzam30.nhakobeam.ui.Copy.t
import io.github.kimzam30.nhakobeam.ui.formatBytes
import io.github.kimzam30.nhakobeam.ui.formatEta
import io.github.kimzam30.nhakobeam.ui.formatSpeed
import io.github.kimzam30.nhakobeam.ui.percent
import io.github.kimzam30.nhakobeam.ui.summarizeFiles
import io.github.kimzam30.nhakobeam.ui.theme.IconWeight
import io.github.kimzam30.nhakobeam.ui.theme.Nhako
import io.github.kimzam30.nhakobeam.ui.theme.PhIcon
import io.github.kimzam30.nhakobeam.ui.theme.style
import kotlinx.coroutines.delay

/** Nearby / Direct / Relayed, with the matching icon. */
@Composable
fun RouteBadge(route: Route, modifier: Modifier = Modifier) {
    val c = Nhako.colors
    when (route) {
        Route.Nearby -> Badge(t("transfer.route.nearby"), "wifi-high", c.accentSubtle, c.accentText, modifier)
        Route.Direct -> Badge(t("transfer.route.direct"), "lightning", c.accentSubtle, c.accentText, modifier)
        Route.Relayed -> Badge(t("transfer.route.relayed"), "share-network", c.fill, c.textSecondary, modifier)
    }
}

enum class BeamMode { Moving, Idle, Paused, Reconnecting, Done, Failed }

/**
 * The beam: the fill follows progress on the smooth spring, a glow follows throughput, and a
 * highlight travels along the filled part faster when the transfer is faster.
 */
@Composable
fun Beam(progress: Float, intensity: Float, mode: BeamMode, modifier: Modifier = Modifier) {
    val c = Nhako.colors
    val reduced = Nhako.reducedMotion
    val p by animateFloatAsState(progress.coerceIn(0f, 1f), if (reduced) tween(0) else NhakoMotion.smooth.spec(), label = "beam")
    val shimmerMs = (2600 - 1700 * intensity.coerceIn(0f, 1f)).toInt()
    val shimmer = rememberInfiniteTransition(label = "shimmer").animateFloat(0f, 1f, infiniteRepeatable(tween(shimmerMs, easing = LinearEasing)), label = "s")
    val glow by animateFloatAsState(if (mode == BeamMode.Moving) 0.35f + 0.65f * intensity.coerceIn(0f, 1f) else 0f, tween(NhakoMotion.Duration.slow), label = "glow")
    val fillColor = when (mode) {
        BeamMode.Paused -> c.textTertiary
        BeamMode.Reconnecting -> c.warning
        BeamMode.Failed -> c.danger
        BeamMode.Done -> c.success
        else -> c.beamCore
    }
    Canvas(modifier.fillMaxWidth().height(14.dp).semantics { progressBarRangeInfo = ProgressBarRangeInfo(p, 0f..1f) }) {
        val h = 6.dp.toPx()
        val top = (size.height - h) / 2
        val r = CornerRadius(h / 2)
        if (glow > 0f) {
            drawRoundRect(c.beamGlow.copy(alpha = c.beamGlow.alpha * glow), Offset(0f, top - 4.dp.toPx()), Size(size.width * p, h + 8.dp.toPx()), CornerRadius(h))
        }
        drawRoundRect(c.beamTrack, Offset(0f, top), Size(size.width, h), r)
        drawRoundRect(fillColor, Offset(0f, top), Size(size.width * p, h), r)
        if (mode == BeamMode.Moving && !reduced && p > 0.02f) {
            val w = 36.dp.toPx()
            val x = shimmer.value * (size.width * p + w) - w
            drawRoundRect(Color.White.copy(alpha = 0.45f), Offset(x.coerceAtLeast(0f), top), Size(w.coerceAtMost(size.width * p - x.coerceAtLeast(0f)).coerceAtLeast(0f), h), r)
        }
    }
}

@Composable
fun TransferCard(app: AppState, tr: Transfer, modifier: Modifier = Modifier, onDetails: (() -> Unit)? = null) {
    val c = Nhako.colors
    val reduced = Nhako.reducedMotion
    val device = app.device(tr.deviceId)
    val name = device?.name ?: "The other device"
    val pct = percent(tr.doneBytes, tr.totalBytes)
    val moving = tr.status == TransferStatus.Transferring
    val ended = tr.status in setOf(TransferStatus.Done, TransferStatus.Failed, TransferStatus.Cancelled, TransferStatus.Declined)
    val unreachable = tr.status == TransferStatus.Failed && tr.error == TransferError.PeerGone && tr.doneBytes == 0L
    val canRetry = tr.status == TransferStatus.Failed || tr.status == TransferStatus.Declined || (tr.status == TransferStatus.Cancelled && tr.cancelledBy == Side.Peer)
    // An ended card with nothing to do but dismiss keeps its close button in the title row.
    val closeInHeader = ended && !canRetry && !(tr.status == TransferStatus.Done && tr.direction == TransferDirection.Receive)

    val title = when (tr.status) {
        TransferStatus.Waiting -> t(if (tr.waitingLong) "send.waiting.long" else "send.waiting", "device" to name)
        TransferStatus.Connecting -> t("transfer.connecting")
        TransferStatus.Done -> t(if (tr.direction == TransferDirection.Send) "transfer.done.sent" else "transfer.done.received", "device" to name)
        TransferStatus.Declined -> t("send.declined", "device" to name)
        TransferStatus.Failed -> if (unreachable) t("send.unreachable", "device" to name) else t("transfer.failed")
        TransferStatus.Cancelled -> if (tr.cancelledBy == Side.Peer) t("transfer.cancelled.byOther", "device" to name) else t("transfer.cancelled.byYou")
        else -> t(if (tr.direction == TransferDirection.Send) "transfer.sending" else "transfer.receiving", "device" to name)
    }
    val detail = when {
        tr.status == TransferStatus.Paused && tr.error == TransferError.DiskFull -> t("transfer.diskFull")
        tr.status == TransferStatus.Paused && tr.error == TransferError.DiskFullPeer -> t("transfer.diskFull.other", "device" to name)
        tr.status == TransferStatus.Paused && tr.pausedBy == Side.Peer -> t("transfer.paused.byOther", "device" to name)
        tr.status == TransferStatus.Paused -> t("transfer.paused")
        tr.status == TransferStatus.Reconnecting -> t("transfer.reconnecting")
        tr.status == TransferStatus.Failed && tr.error != null && !unreachable ->
            t("error.${errorKey(tr.error)}", "device" to name, "fileName" to (tr.files.firstOrNull()?.name ?: "A file"))
        else -> "${summarizeFiles(tr.files)} · ${formatBytes(tr.totalBytes)}"
    }
    val detailColor = when {
        tr.status == TransferStatus.Failed -> c.danger
        tr.status == TransferStatus.Reconnecting || tr.error == TransferError.DiskFull || tr.error == TransferError.DiskFullPeer -> if (Nhako.dark) c.warning else Color(0xFF8A5300)
        else -> c.textSecondary
    }
    val mode = when (tr.status) {
        TransferStatus.Transferring -> BeamMode.Moving
        TransferStatus.Paused -> BeamMode.Paused
        TransferStatus.Reconnecting -> BeamMode.Reconnecting
        TransferStatus.Done -> BeamMode.Done
        TransferStatus.Failed -> BeamMode.Failed
        else -> BeamMode.Idle
    }

    // Failure: one small shake. Done: tuck away into history after 6 s.
    val shake = remember { Animatable(0f) }
    LaunchedEffect(tr.status) {
        if (tr.status == TransferStatus.Failed && !reduced) {
            for (x in listOf(-8f, 7f, -5f, 3f, 0f)) shake.animateTo(x, tween(55))
        }
        if (tr.status == TransferStatus.Done) {
            delay(6000)
            app.dismissTransfer(tr.id)
        }
    }

    NCard(modifier.graphicsLayer { translationX = shake.value * density }) {
        Column(verticalArrangement = Arrangement.spacedBy(NhakoSpace.s3)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Box(Modifier.size(40.dp).clip(CircleShape).background(if (tr.status == TransferStatus.Done) c.successSubtle else c.accentSubtle), contentAlignment = Alignment.Center) {
                    if (tr.status == TransferStatus.Done) {
                        val pop = remember { Animatable(0.3f) }
                        LaunchedEffect(Unit) { pop.animateTo(1f, NhakoMotion.bouncy.spec()) }
                        PhIcon("check-circle", Modifier.graphicsLayer { scaleX = pop.value; scaleY = pop.value }, size = 22.dp, tint = c.success, weight = IconWeight.Fill)
                    } else {
                        PhIcon(if (tr.direction == TransferDirection.Send) kindIcon(device?.kind ?: io.github.kimzam30.nhakobeam.model.DeviceKind.Laptop) else "download-simple", size = 20.dp, tint = c.accent)
                    }
                }
                Spacer(Modifier.width(NhakoSpace.s4))
                Column(Modifier.weight(1f)) {
                    BasicText(title, style = NhakoType.headline.style(c.textPrimary), maxLines = 2, overflow = TextOverflow.Ellipsis)
                    BasicText(detail, style = NhakoType.footnote.style(detailColor), maxLines = 3, overflow = TextOverflow.Ellipsis)
                }
                if (closeInHeader) NIconButton("x", t("transfer.dismiss"), { app.dismissTransfer(tr.id) })
            }
            if (tr.status != TransferStatus.Waiting && tr.status != TransferStatus.Declined && !(tr.status == TransferStatus.Failed && tr.doneBytes == 0L) && tr.status != TransferStatus.Done) {
                Beam(if (tr.totalBytes > 0) tr.doneBytes.toFloat() / tr.totalBytes else 0f, (tr.speed / 92e6).toFloat(), mode)
            }
            if (!closeInHeader) Row(verticalAlignment = Alignment.CenterVertically) {
                if (tr.status in setOf(TransferStatus.Transferring, TransferStatus.Paused, TransferStatus.Reconnecting, TransferStatus.Connecting)) {
                    Column(Modifier.weight(1f)) {
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            BasicText("$pct%", style = NhakoType.callout.style(c.textPrimary).copy(fontWeight = FontWeight(600)))
                            if (moving && tr.speed > 0) {
                                Spacer(Modifier.width(NhakoSpace.s4))
                                BasicText(formatSpeed(tr.speed), style = NhakoType.callout.style(c.textSecondary))
                            }
                        }
                        if (moving) {
                            val showEta = tr.movingSince != null && System.currentTimeMillis() - tr.movingSince > 3000
                            val eta = formatEta(tr.totalBytes - tr.doneBytes, tr.speed)
                            BasicText(if (showEta && eta != null) t("transfer.eta", "time" to eta) else t("transfer.eta.calculating"), style = NhakoType.footnote.style(c.textSecondary))
                        }
                        Spacer(Modifier.height(NhakoSpace.s2))
                        RouteBadge(tr.route)
                    }
                    if (tr.status == TransferStatus.Paused && tr.pausedBy != Side.Peer) {
                        NIconButton("play", t("transfer.resume"), { app.engine.resume(tr.id) }, tint = c.accent, weight = IconWeight.Fill)
                    } else if (tr.status == TransferStatus.Transferring) {
                        NIconButton("pause", t("transfer.pause"), { app.engine.pause(tr.id) }, tint = c.textPrimary, weight = IconWeight.Fill)
                    }
                    if (onDetails != null) NIconButton("files", t("transfer.details"), onDetails)
                    NIconButton("x", t("send.cancel"), { app.engine.cancel(tr.id) })
                } else if (tr.status == TransferStatus.Waiting) {
                    Spacer(Modifier.weight(1f))
                    NButton(t("send.cancel"), { app.engine.cancel(tr.id) }, variant = ButtonVariant.Plain, size = ButtonSize.Sm)
                } else if (tr.status == TransferStatus.Done) {
                    if (tr.direction == TransferDirection.Receive) {
                        NButton(t("transfer.done.open"), { app.openReceived(tr) }, variant = ButtonVariant.Secondary, size = ButtonSize.Sm, icon = "arrow-square-out")
                        Spacer(Modifier.width(NhakoSpace.s3))
                        NButton(t("transfer.done.share"), { app.openReceived(tr, share = true) }, variant = ButtonVariant.Plain, size = ButtonSize.Sm, icon = "share-network")
                    }
                    Spacer(Modifier.weight(1f))
                    NIconButton("x", t("transfer.dismiss"), { app.dismissTransfer(tr.id) })
                } else if (ended) {
                    if (canRetry) {
                        NButton(t("transfer.failed.retry"), { app.engine.retry(tr.id) }, variant = ButtonVariant.Secondary, size = ButtonSize.Sm, icon = "arrow-clockwise")
                    }
                    Spacer(Modifier.weight(1f))
                    NIconButton("x", t("transfer.dismiss"), { app.dismissTransfer(tr.id) })
                }
            }
        }
    }
}

private fun errorKey(e: TransferError) = when (e) {
    TransferError.PeerGone -> "peerGone"
    TransferError.Network -> "network"
    TransferError.FileChanged -> "fileChanged"
    TransferError.Integrity -> "integrity"
    TransferError.RelayUnreachable -> "relayUnreachable"
    TransferError.Permission -> "permission"
    TransferError.DiskFull -> "diskFull"
    TransferError.DiskFullPeer -> "diskFullPeer"
    TransferError.Unknown -> "unknown"
}
