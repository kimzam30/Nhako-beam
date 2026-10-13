// History: every transfer that ended, grouped by day, newest first, and the details sheet for one.
// The Compose twin of desktop/src/screens/History.svelte. docs/design/screens-and-states.md, section 7.
package io.github.kimzam30.nhakobeam.ui.screens

import android.text.format.DateFormat
import androidx.activity.compose.BackHandler
import androidx.compose.animation.core.Animatable
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import io.github.kimzam30.nhakobeam.design.NhakoMotion
import io.github.kimzam30.nhakobeam.design.NhakoRadius
import io.github.kimzam30.nhakobeam.design.NhakoSpace
import io.github.kimzam30.nhakobeam.design.NhakoType
import io.github.kimzam30.nhakobeam.model.HistoryEntry
import io.github.kimzam30.nhakobeam.model.Side
import io.github.kimzam30.nhakobeam.model.TransferDirection
import io.github.kimzam30.nhakobeam.model.TransferError
import io.github.kimzam30.nhakobeam.model.TransferStatus
import io.github.kimzam30.nhakobeam.state.AppState
import io.github.kimzam30.nhakobeam.state.Screen
import io.github.kimzam30.nhakobeam.state.Sheet
import io.github.kimzam30.nhakobeam.ui.Copy.t
import io.github.kimzam30.nhakobeam.ui.components.Bolt
import io.github.kimzam30.nhakobeam.ui.components.BoltMode
import io.github.kimzam30.nhakobeam.ui.components.ButtonSize
import io.github.kimzam30.nhakobeam.ui.components.ButtonVariant
import io.github.kimzam30.nhakobeam.ui.components.FILES_PAGE
import io.github.kimzam30.nhakobeam.ui.components.MoreFiles
import io.github.kimzam30.nhakobeam.ui.components.NButton
import io.github.kimzam30.nhakobeam.ui.components.NIconButton
import io.github.kimzam30.nhakobeam.ui.components.RouteBadge
import io.github.kimzam30.nhakobeam.ui.components.Separator
import io.github.kimzam30.nhakobeam.ui.components.pressScale
import io.github.kimzam30.nhakobeam.ui.dayLabel
import io.github.kimzam30.nhakobeam.ui.dayStart
import io.github.kimzam30.nhakobeam.ui.formatBytes
import io.github.kimzam30.nhakobeam.ui.formatDuration
import io.github.kimzam30.nhakobeam.ui.formatSpeed
import io.github.kimzam30.nhakobeam.ui.isLink
import io.github.kimzam30.nhakobeam.ui.middleTruncate
import io.github.kimzam30.nhakobeam.ui.summarize
import io.github.kimzam30.nhakobeam.ui.summarizeFiles
import io.github.kimzam30.nhakobeam.ui.theme.IconWeight
import io.github.kimzam30.nhakobeam.ui.theme.Nhako
import io.github.kimzam30.nhakobeam.ui.theme.PhIcon
import io.github.kimzam30.nhakobeam.ui.theme.style
import io.github.kimzam30.nhakobeam.ui.timeOfDay
import io.github.kimzam30.nhakobeam.ui.withTap
import kotlinx.coroutines.delay

/** [now] decides what "Today" is (fixed in the screenshot tests). */
@Composable
fun HistoryScreen(app: AppState, now: Long = System.currentTimeMillis()) {
    val c = Nhako.colors
    BackHandler { app.navigate(Screen.Home) }
    val is24 = DateFormat.is24HourFormat(LocalContext.current)
    val groups = app.history.groupBy { dayStart(it.finishedAt) }.toList()
    // Rows come in one after another the first time; later ones just appear.
    val order = remember { mutableMapOf<String, Int>() }
    app.history.forEach { order.getOrPut(it.id) { order.size } }

    Column(Modifier.fillMaxSize().background(c.canvas).windowInsetsPadding(WindowInsets.statusBars)) {
        Row(Modifier.padding(start = NhakoSpace.s3, end = NhakoSpace.s4, top = NhakoSpace.s3), verticalAlignment = Alignment.CenterVertically) {
            NIconButton("caret-left", t("settings.back"), { app.navigate(Screen.Home) }, size = 24.dp, weight = IconWeight.Bold)
            BasicText(t("history.title"), Modifier.weight(1f), style = NhakoType.title1.style(c.textPrimary))
            if (app.history.isNotEmpty()) {
                NButton(t("history.clear"), { app.sheet = Sheet.ConfirmClearHistory }, variant = ButtonVariant.Plain, size = ButtonSize.Sm)
            }
        }
        if (app.history.isEmpty()) {
            Rise(0, Modifier.fillMaxWidth().padding(top = NhakoSpace.s9)) {
                Column(Modifier.fillMaxWidth().padding(horizontal = NhakoSpace.s7), horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(NhakoSpace.s4)) {
                    Box(Modifier.size(64.dp).clip(CircleShape).background(c.fill), contentAlignment = Alignment.Center) {
                        // A sleeping bolt: it breathes once when you arrive, then rests.
                        Bolt(34.dp, mode = BoltMode.Asleep)
                    }
                    BasicText(t("history.empty"), style = NhakoType.callout.style(c.textSecondary).copy(textAlign = androidx.compose.ui.text.style.TextAlign.Center))
                }
            }
            return@Column
        }
        LazyColumn(
            Modifier.fillMaxSize(),
            contentPadding = PaddingValues(
                start = NhakoSpace.s5, end = NhakoSpace.s5, top = NhakoSpace.s5,
                bottom = NhakoSpace.s9 + WindowInsets.navigationBars.asPaddingValues().calculateBottomPadding(),
            ),
        ) {
            groups.forEachIndexed { g, (day, entries) ->
                item(key = "day-$day") {
                    Rise(order[entries.first().id] ?: 0, Modifier.animateItem().padding(top = if (g > 0) NhakoSpace.s6 else 0.dp)) {
                        BasicText(
                            dayLabel(day, now, t("history.today"), t("history.yesterday")),
                            Modifier.padding(start = NhakoSpace.s2, bottom = NhakoSpace.s3),
                            style = NhakoType.footnote.style(c.textSecondary).copy(fontWeight = FontWeight(600)),
                        )
                    }
                }
                items(entries, key = { it.id }) { e ->
                    val first = e.id == entries.first().id
                    val last = e.id == entries.last().id
                    val shape = RoundedCornerShape(
                        topStart = if (first) NhakoRadius.lg else 0.dp, topEnd = if (first) NhakoRadius.lg else 0.dp,
                        bottomStart = if (last) NhakoRadius.lg else 0.dp, bottomEnd = if (last) NhakoRadius.lg else 0.dp,
                    )
                    Rise(order[e.id] ?: 0, Modifier.animateItem()) {
                        Column(Modifier.widthIn(max = 640.dp).clip(shape).background(c.surface)) {
                            if (!first) Separator(Modifier.padding(start = NhakoSpace.s5 + 36.dp + NhakoSpace.s4))
                            HistoryRow(e, is24) { app.sheet = if (e.text != null && e.status == io.github.kimzam30.nhakobeam.model.TransferStatus.Done) Sheet.TextSheet(e.id) else Sheet.HistoryItem(e.id) }
                        }
                    }
                }
            }
        }
    }
}

/** Rises 12 dp and fades in, [order] steps late (the stagger stops after a few rows). */
@Composable
private fun Rise(order: Int, modifier: Modifier = Modifier, content: @Composable () -> Unit) {
    val reduced = Nhako.reducedMotion
    val enter = remember { Animatable(if (reduced) 1f else 0f) }
    LaunchedEffect(Unit) {
        delay((order.coerceAtMost(NhakoMotion.staggerMaxItems) * NhakoMotion.staggerStepMs).toLong())
        enter.animateTo(1f, NhakoMotion.smooth.spec())
    }
    Box(modifier.graphicsLayer { translationY = (1 - enter.value) * 12.dp.toPx(); alpha = enter.value.coerceIn(0f, 1f) }) { content() }
}

private fun statusText(e: HistoryEntry): String? = when (e.status) {
    TransferStatus.Failed -> t("history.status.failed")
    TransferStatus.Cancelled -> t("history.status.cancelled")
    TransferStatus.Declined -> t("history.status.declined")
    else -> null
}

@Composable
private fun HistoryRow(e: HistoryEntry, is24: Boolean, onClick: () -> Unit) {
    val c = Nhako.colors
    val press = remember { MutableInteractionSource() }
    val status = statusText(e)
    val receive = e.direction == TransferDirection.Receive
    val (bg, fg) = when {
        status != null -> c.fill to c.textTertiary
        receive -> c.successSubtle to c.success
        else -> c.accentSubtle to c.accentText
    }
    Row(
        Modifier.fillMaxWidth().pressScale(press).clickable(press, null, role = Role.Button, onClick = withTap(onClick))
            .padding(horizontal = NhakoSpace.s5, vertical = NhakoSpace.s3),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(Modifier.size(36.dp).clip(CircleShape).background(bg), contentAlignment = Alignment.Center) {
            PhIcon(if (receive) "download-simple" else "upload-simple", size = 18.dp, tint = fg)
        }
        Spacer(Modifier.width(NhakoSpace.s4))
        Column(Modifier.weight(1f)) {
            BasicText(summarize(e.files, e.text), style = NhakoType.body.style(c.textPrimary).copy(fontWeight = FontWeight(500)), maxLines = 1, overflow = TextOverflow.Ellipsis)
            BasicText(
                "${t(if (receive) "history.received" else "history.sent", "device" to e.deviceName)} · ${e.text?.let { t(if (isLink(it)) "staged.link" else "staged.text") } ?: formatBytes(e.totalBytes)}",
                style = NhakoType.footnote.style(c.textSecondary), maxLines = 1, overflow = TextOverflow.Ellipsis,
            )
        }
        Spacer(Modifier.width(NhakoSpace.s3))
        Column(horizontalAlignment = Alignment.End) {
            BasicText(timeOfDay(e.finishedAt, is24), style = NhakoType.footnote.style(c.textSecondary))
            if (status != null) {
                BasicText(status, style = NhakoType.caption.style(if (e.status == TransferStatus.Failed) c.danger else c.textTertiary).copy(fontWeight = FontWeight(600)))
            }
        }
    }
}

/** The details sheet's content: how it ended, when, with whom, how fast, and the files. */
@Composable
fun ColumnScope.HistoryDetails(e: HistoryEntry, now: Long = System.currentTimeMillis()) {
    val c = Nhako.colors
    val is24 = DateFormat.is24HourFormat(LocalContext.current)
    val done = e.status == TransferStatus.Done
    val headline = when (e.status) {
        TransferStatus.Done -> t(if (e.direction == TransferDirection.Send) "transfer.done.sent" else "transfer.done.received", "device" to e.deviceName)
        TransferStatus.Declined -> t(if (e.error == TransferError.NotAllowed) "send.notAllowed" else "send.declined", "device" to e.deviceName)
        TransferStatus.Cancelled -> if (e.cancelledBy == Side.Peer) t("transfer.cancelled.byOther", "device" to e.deviceName) else t("transfer.cancelled.byYou")
        else -> t("history.status.failed")
    }
    Row(verticalAlignment = Alignment.CenterVertically) {
        PhIcon(
            when (e.status) {
                TransferStatus.Done -> "check-circle"
                TransferStatus.Failed -> "warning-circle"
                else -> "x-circle"
            },
            size = 18.dp, tint = if (done) c.success else c.textSecondary, weight = IconWeight.Fill,
        )
        Spacer(Modifier.width(NhakoSpace.s2))
        BasicText(headline, style = NhakoType.callout.style(if (done) c.success else c.textSecondary).copy(fontWeight = FontWeight(500)))
    }
    Spacer(Modifier.height(NhakoSpace.s4))
    var i = 0
    Fact(i++, t("history.detail.when"), "${dayLabel(e.finishedAt, now, t("history.today"), t("history.yesterday"))}, ${timeOfDay(e.finishedAt, is24)}")
    Fact(i++, t("history.detail.device"), e.deviceName)
    Fact(i++, t("history.detail.route"), null) { RouteBadge(e.route) }
    Fact(
        i++, t("history.detail.size"),
        if (e.doneBytes < e.totalBytes) t("history.detail.partial", "done" to formatBytes(e.doneBytes), "total" to formatBytes(e.totalBytes)) else formatBytes(e.totalBytes),
    )
    if (e.doneBytes > 0) Fact(i++, t("history.detail.duration"), formatDuration(e.finishedAt - e.startedAt))
    if (e.averageSpeed > 0) Fact(i, t("history.detail.speed"), formatSpeed(e.averageSpeed))
    Spacer(Modifier.height(NhakoSpace.s5))
    BasicText(t("transfer.files"), style = NhakoType.footnote.style(c.textSecondary).copy(fontWeight = FontWeight(600)))
    Spacer(Modifier.height(NhakoSpace.s2))
    var shown by remember(e.id) { mutableIntStateOf(FILES_PAGE) }
    e.files.take(shown).forEach { f ->
        Row(Modifier.fillMaxWidth().padding(vertical = NhakoSpace.s2)) {
            FileName(f, Modifier.weight(1f))
            BasicText(formatBytes(f.size), style = NhakoType.callout.style(c.textSecondary))
        }
    }
    MoreFiles(e.files.size, shown) { shown += FILES_PAGE }
}

@Composable
private fun Fact(order: Int, label: String, value: String?, content: (@Composable () -> Unit)? = null) {
    val c = Nhako.colors
    Rise(order) {
        Column {
            Row(Modifier.fillMaxWidth().height(44.dp), verticalAlignment = Alignment.CenterVertically) {
                BasicText(label, Modifier.weight(1f), style = NhakoType.callout.style(c.textSecondary))
                if (value != null) BasicText(value, style = NhakoType.callout.style(c.textPrimary), maxLines = 1, overflow = TextOverflow.Ellipsis)
                content?.invoke()
            }
            Separator()
        }
    }
}
