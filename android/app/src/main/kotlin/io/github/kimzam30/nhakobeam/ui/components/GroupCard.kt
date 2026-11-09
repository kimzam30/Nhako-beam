package io.github.kimzam30.nhakobeam.ui.components

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import io.github.kimzam30.nhakobeam.design.NhakoMotion
import io.github.kimzam30.nhakobeam.design.NhakoRadius
import io.github.kimzam30.nhakobeam.design.NhakoSpace
import io.github.kimzam30.nhakobeam.design.NhakoType
import io.github.kimzam30.nhakobeam.model.Transfer
import io.github.kimzam30.nhakobeam.model.TransferStatus
import io.github.kimzam30.nhakobeam.state.AppState
import io.github.kimzam30.nhakobeam.state.Sheet
import io.github.kimzam30.nhakobeam.state.groupProgress
import io.github.kimzam30.nhakobeam.ui.Copy.t
import io.github.kimzam30.nhakobeam.ui.formatBytes
import io.github.kimzam30.nhakobeam.ui.formatSpeed
import io.github.kimzam30.nhakobeam.ui.percent
import io.github.kimzam30.nhakobeam.ui.summarizeFiles
import io.github.kimzam30.nhakobeam.ui.textPreview
import io.github.kimzam30.nhakobeam.ui.theme.Nhako
import io.github.kimzam30.nhakobeam.ui.theme.style
import kotlinx.coroutines.delay

private val ENDED = setOf(TransferStatus.Done, TransferStatus.Failed, TransferStatus.Cancelled, TransferStatus.Declined)

/**
 * A send to several devices: one card, a row per device. Each device accepts or declines on its
 * own; a row opens that transfer's details. The desktop draws the same card (GroupCard.svelte).
 */
@Composable
fun GroupCard(app: AppState, id: String, transfers: List<Transfer>, modifier: Modifier = Modifier) {
    val c = Nhako.colors
    val first = transfers.first()
    val text = first.text
    val allEnded = transfers.all { it.status in ENDED }
    val allDone = transfers.all { it.status == TransferStatus.Done }
    val what = if (text != null) t("group.text") else summarizeFiles(first.files)
    val title = t(if (allDone) "group.sent" else if (allEnded) "group.ended" else "group.sending", "what" to what, "count" to transfers.size)
    val pct = (groupProgress(transfers) * 100).toInt()

    // All done: fold into history after 6 s, like a single transfer.
    LaunchedEffect(allDone) {
        if (allDone) {
            delay(6000)
            app.dismissGroup(id)
        }
    }

    NCard(modifier) {
        Column(verticalArrangement = Arrangement.spacedBy(NhakoSpace.s3)) {
            Row(verticalAlignment = Alignment.Top) {
                Column(Modifier.weight(1f)) {
                    BasicText(title, Modifier.semantics { liveRegion = LiveRegionMode.Polite }, style = NhakoType.headline.style(c.textPrimary))
                    BasicText(
                        if (text != null) textPreview(text) else "${summarizeFiles(first.files)} · ${formatBytes(first.totalBytes)}",
                        style = NhakoType.footnote.style(c.textSecondary), maxLines = 1, overflow = TextOverflow.Ellipsis,
                    )
                }
                if (!allEnded) BasicText("$pct%", Modifier.padding(start = NhakoSpace.s3), style = NhakoType.footnote.style(c.textSecondary))
                else NIconButton("x", t("group.dismiss"), { app.dismissGroup(id) })
            }
            Column {
                for (x in transfers) androidx.compose.runtime.key(x.id) { GroupRow(app, x) }
            }
            if (!allEnded) {
                BasicText(t("group.hint"), style = NhakoType.footnote.style(c.textSecondary))
                NButton(t("group.cancelAll"), { app.cancelGroup(id) }, variant = ButtonVariant.Plain, size = ButtonSize.Sm, sound = io.github.kimzam30.nhakobeam.design.NhakoSound.Cancel)
            }
        }
    }
}

@Composable
private fun GroupRow(app: AppState, x: Transfer) {
    val c = Nhako.colors
    val reduced = Nhako.reducedMotion
    val state = when (x.status) {
        TransferStatus.Transferring -> t("group.row.moving", "percent" to percent(x.doneBytes, x.totalBytes), "speed" to formatSpeed(x.speed))
        TransferStatus.Waiting -> t("group.row.waiting")
        TransferStatus.Connecting -> t("group.row.connecting")
        TransferStatus.Paused -> t("group.row.paused")
        TransferStatus.Reconnecting -> t("group.row.reconnecting")
        TransferStatus.Done -> t("group.row.done")
        TransferStatus.Declined -> t("group.row.declined")
        TransferStatus.Failed -> t("group.row.failed")
        TransferStatus.Cancelled -> t("group.row.cancelled")
    }
    val stateColor = when (x.status) {
        TransferStatus.Done -> c.success
        TransferStatus.Failed, TransferStatus.Declined -> c.danger
        else -> c.textSecondary
    }
    val target = if (x.status == TransferStatus.Done) 1f else percent(x.doneBytes, x.totalBytes) / 100f
    val fill by animateFloatAsState(target, if (reduced) tween(1) else tween(NhakoMotion.Duration.base), label = "groupFill")
    val fillColor by animateColorAsState(
        when (x.status) {
            TransferStatus.Done -> c.success
            TransferStatus.Failed, TransferStatus.Declined, TransferStatus.Cancelled -> c.textTertiary
            else -> c.accent
        },
        tween(NhakoMotion.Duration.fast), label = "groupFillColor",
    )
    Column(
        Modifier.fillMaxWidth().heightIn(min = 48.dp).clip(RoundedCornerShape(NhakoRadius.sm))
            .clickable(role = Role.Button) { app.sheet = Sheet.TransferDetails(x.id) }
            .padding(vertical = NhakoSpace.s2, horizontal = NhakoSpace.s1),
        verticalArrangement = Arrangement.spacedBy(6.dp),
    ) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            BasicText(app.deviceName(x.deviceId), Modifier.weight(1f), style = NhakoType.footnote.style(c.textPrimary).copy(fontWeight = FontWeight(500)), maxLines = 1, overflow = TextOverflow.Ellipsis)
            BasicText(state, Modifier.padding(start = NhakoSpace.s3), style = NhakoType.footnote.style(stateColor))
        }
        Box(Modifier.fillMaxWidth().height(6.dp).clip(CircleShape).background(c.beamTrack)) {
            Box(Modifier.fillMaxWidth(fill.coerceIn(0f, 1f)).fillMaxHeight().clip(CircleShape).background(fillColor))
        }
    }
}
