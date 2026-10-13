// Every bottom sheet: choose a device, show a code, enter a code, scan, paired, transfer files,
// remove a trusted device. The Compose twin of desktop/src/components/Sheets.svelte.
package io.github.kimzam30.nhakobeam.ui.screens

import androidx.activity.compose.BackHandler
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawWithContent
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import io.github.kimzam30.nhakobeam.design.NhakoHaptic
import io.github.kimzam30.nhakobeam.design.NhakoMotion
import io.github.kimzam30.nhakobeam.design.NhakoRadius
import io.github.kimzam30.nhakobeam.design.NhakoSound
import io.github.kimzam30.nhakobeam.design.NhakoSpace
import io.github.kimzam30.nhakobeam.design.NhakoType
import io.github.kimzam30.nhakobeam.model.Device
import io.github.kimzam30.nhakobeam.state.AppState
import io.github.kimzam30.nhakobeam.state.Sheet
import io.github.kimzam30.nhakobeam.ui.Copy.t
import io.github.kimzam30.nhakobeam.ui.components.Bolt
import io.github.kimzam30.nhakobeam.ui.components.ButtonSize
import io.github.kimzam30.nhakobeam.ui.components.ButtonVariant
import io.github.kimzam30.nhakobeam.ui.components.CodeDisplay
import io.github.kimzam30.nhakobeam.ui.components.CodeInput
import io.github.kimzam30.nhakobeam.ui.components.CodeInputState
import io.github.kimzam30.nhakobeam.ui.components.DeviceOrb
import io.github.kimzam30.nhakobeam.ui.components.FILES_PAGE
import io.github.kimzam30.nhakobeam.ui.components.FileStack
import io.github.kimzam30.nhakobeam.ui.components.MoreFiles
import io.github.kimzam30.nhakobeam.ui.components.NButton
import io.github.kimzam30.nhakobeam.ui.components.NSheet
import io.github.kimzam30.nhakobeam.ui.components.NTextArea
import io.github.kimzam30.nhakobeam.ui.components.NToggle
import io.github.kimzam30.nhakobeam.ui.components.QrScanner
import io.github.kimzam30.nhakobeam.ui.components.RouteBadge
import io.github.kimzam30.nhakobeam.ui.components.Spinner
import io.github.kimzam30.nhakobeam.ui.components.sparks
import io.github.kimzam30.nhakobeam.ui.formatBytes
import io.github.kimzam30.nhakobeam.ui.isLink
import io.github.kimzam30.nhakobeam.ui.middleTruncate
import io.github.kimzam30.nhakobeam.ui.percent
import io.github.kimzam30.nhakobeam.ui.summarizeFiles
import io.github.kimzam30.nhakobeam.ui.textPreview
import io.github.kimzam30.nhakobeam.ui.theme.IconWeight
import io.github.kimzam30.nhakobeam.ui.theme.Nhako
import io.github.kimzam30.nhakobeam.ui.theme.PhIcon
import io.github.kimzam30.nhakobeam.ui.theme.style
import io.github.kimzam30.nhakobeam.ui.withTap
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

/**
 * Hosts whichever sheet is open. The last one stays on screen while it slides away, so closing is
 * animated too.
 */
@Composable
fun SheetHost(app: AppState) {
    var shown by remember { mutableStateOf<Sheet?>(null) }
    val current = app.sheet
    if (current != null) shown = current
    val sheet = shown ?: return
    val visible = current != null

    fun close() {
        if (app.sheet == Sheet.ShowCode) app.engine.cancelCode()
        app.sheet = null
    }
    BackHandler(enabled = visible) { close() }

    when (sheet) {
        Sheet.Send -> NSheet(visible, t("send.choose.title"), ::close) { SendContent(app) }
        Sheet.ShowCode -> NSheet(visible, t("code.show.title"), ::close) { ShowCodeContent(app) }
        Sheet.EnterCode -> EnterCodeSheet(app, visible, ::close)
        Sheet.Scan -> NSheet(visible, t("code.scan.title"), ::close, footer = {
            NButton(t("code.scan.type"), { app.sheet = Sheet.EnterCode }, variant = ButtonVariant.Secondary, icon = "keyboard", full = true)
        }) {
            ScanContent(app)
        }
        is Sheet.Paired -> PairedSheet(app, sheet, visible)
        is Sheet.TransferDetails -> NSheet(visible, t("transfer.files"), ::close) { DetailsContent(app, sheet.id) }
        is Sheet.ConfirmRemove -> {
            val d = app.device(sheet.deviceId)
            NSheet(visible, t("settings.trusted.remove"), ::close, footer = {
                Row(horizontalArrangement = Arrangement.spacedBy(NhakoSpace.s3)) {
                    NButton(t("common.cancel"), ::close, Modifier.weight(1f), variant = ButtonVariant.Secondary, full = true)
                    NButton(t("settings.trusted.remove"), {
                        app.engine.setTrusted(sheet.deviceId, false)
                        app.sheet = null
                    }, Modifier.weight(1f), variant = ButtonVariant.Danger, full = true)
                }
            }) {
                BasicText(t("settings.trusted.remove.confirm", "device" to (d?.name ?: "this device")), style = NhakoType.body.style(Nhako.colors.textSecondary))
            }
        }
        is Sheet.HistoryItem -> {
            val e = app.historyEntry(sheet.id)
            if (e == null) {
                LaunchedEffect(Unit) { app.sheet = null }
                return
            }
            val canOpen = e.direction == io.github.kimzam30.nhakobeam.model.TransferDirection.Receive && e.status == io.github.kimzam30.nhakobeam.model.TransferStatus.Done
            NSheet(visible, summarizeFiles(e.files), ::close, footer = if (!canOpen) null else ({
                Row(horizontalArrangement = Arrangement.spacedBy(NhakoSpace.s3)) {
                    NButton(t("transfer.done.share"), { app.openPast(e, share = true) }, Modifier.weight(1f), variant = ButtonVariant.Secondary, full = true, icon = "share-network")
                    NButton(t("transfer.done.open"), { app.openPast(e) }, Modifier.weight(1f), full = true, icon = "arrow-square-out")
                }
            })) { HistoryDetails(e) }
        }
        is Sheet.ConfirmOpen -> NSheet(visible, t("open.risky.title", "file" to middleTruncate(sheet.fileName, 28)), ::close, footer = {
            Row(horizontalArrangement = Arrangement.spacedBy(NhakoSpace.s3)) {
                NButton(t("common.cancel"), ::close, Modifier.weight(1f), variant = ButtonVariant.Secondary, full = true)
                NButton(t("open.risky.confirm"), {
                    if (sheet.past) app.historyEntry(sheet.id)?.let { app.openPast(it, confirmed = true) }
                    else app.transfers.find { it.id == sheet.id }?.let { app.openReceived(it, confirmed = true) }
                    app.sheet = null
                }, Modifier.weight(1f), variant = ButtonVariant.Danger, full = true)
            }
        }) { RiskyOpen(sheet) }
        is Sheet.Compose -> ComposeSheet(app, sheet, visible, ::close)
        is Sheet.DeviceSheet -> {
            val d = app.device(sheet.deviceId)
            if (d == null) {
                LaunchedEffect(Unit) { app.sheet = null }
                return
            }
            NSheet(visible, d.name, ::close) { DeviceContent(app, d) }
        }
        is Sheet.TrustWait -> {
            val name = app.deviceName(sheet.deviceId)
            NSheet(visible, t("trust.wait.title", "device" to name), { app.cancelTrust(sheet.deviceId) }, footer = {
                NButton(t("common.cancel"), { app.cancelTrust(sheet.deviceId) }, variant = ButtonVariant.Secondary, full = true)
            }) {
                TrustPair(app, sheet.deviceId, live = sheet.check != null, waiting = true)
                if (sheet.check != null) CheckChip(sheet, t("trust.wait.body", "device" to name), sheet.check)
                else BasicText(t("trust.wait.reaching", "device" to name), Modifier.fillMaxWidth(), style = NhakoType.body.style(Nhako.colors.textSecondary).copy(textAlign = androidx.compose.ui.text.style.TextAlign.Center))
            }
        }
        is Sheet.TrustAsk -> {
            val name = app.deviceName(sheet.deviceId)
            NSheet(visible, t("trust.ask.title", "device" to name), { app.answerTrust(sheet.deviceId, false) }, footer = {
                Row(horizontalArrangement = Arrangement.spacedBy(NhakoSpace.s3)) {
                    NButton(t("trust.ask.no"), { app.answerTrust(sheet.deviceId, false) }, Modifier.weight(1f), variant = ButtonVariant.Secondary, full = true)
                    NButton(t("trust.ask.yes"), { app.answerTrust(sheet.deviceId, true) }, Modifier.weight(1f), full = true, icon = "handshake")
                }
            }) {
                TrustPair(app, sheet.deviceId, live = true, waiting = false)
                BasicText(t("trust.ask.body", "device" to name), style = NhakoType.body.style(Nhako.colors.textSecondary))
                CheckChip(sheet, t("trust.ask.check"), sheet.check)
            }
        }
        is Sheet.TextSheet -> {
            val live = app.transfers.find { it.id == sheet.id }
            val past = app.historyEntry(sheet.id)
            val text = live?.text ?: past?.text
            if (text == null) {
                LaunchedEffect(Unit) { app.sheet = null }
                return
            }
            val sent = (live?.direction ?: past?.direction) == io.github.kimzam30.nhakobeam.model.TransferDirection.Send
            val device = live?.let { app.deviceName(it.deviceId) } ?: past?.deviceName.orEmpty()
            val link = isLink(text)
            val key = if (sent) (if (link) "text.link.to" else "text.to") else if (link) "text.link.from" else "text.from"
            NSheet(visible, t(key, "device" to device), ::close, footer = {
                Row(horizontalArrangement = Arrangement.spacedBy(NhakoSpace.s3)) {
                    NButton(t("text.copy"), { app.copyText(text) }, Modifier.weight(1f), variant = if (link) ButtonVariant.Secondary else ButtonVariant.Primary, full = true, icon = "copy")
                    if (link) NButton(t("text.open"), { app.openLink(text) }, Modifier.weight(1f), full = true, icon = "arrow-square-out")
                }
            }) {
                androidx.compose.foundation.text.selection.SelectionContainer {
                    BasicText(text, Modifier.fillMaxWidth().clip(RoundedCornerShape(NhakoRadius.md)).background(Nhako.colors.surfaceSunken).padding(NhakoSpace.s4), style = NhakoType.body.style(Nhako.colors.textPrimary))
                }
            }
        }
        Sheet.ConfirmClearHistory -> NSheet(visible, t("history.clear"), ::close, footer = {
            Row(horizontalArrangement = Arrangement.spacedBy(NhakoSpace.s3)) {
                NButton(t("common.cancel"), ::close, Modifier.weight(1f), variant = ButtonVariant.Secondary, full = true)
                NButton(t("history.clear"), { app.clearHistory() }, Modifier.weight(1f), variant = ButtonVariant.Danger, full = true)
            }
        }) {
            BasicText(t("history.clear.confirm"), style = NhakoType.body.style(Nhako.colors.textSecondary))
        }
    }
}

@Composable
private fun SendContent(app: AppState) {
    val c = Nhako.colors
    val trusted = app.devices.filter { it.trusted }.sortedByDescending { it.reachable }
    val nearby = app.devices.filter { !it.trusted && it.reachable }
    val text = app.stagedText
    if (text != null) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            FileStack(emptyList(), 40.dp, text = text)
            Spacer(Modifier.width(NhakoSpace.s4))
            BasicText(textPreview(text), style = NhakoType.callout.style(c.textPrimary).copy(fontWeight = FontWeight(500)), maxLines = 1, overflow = TextOverflow.Ellipsis)
        }
    } else if (app.staged.isNotEmpty()) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            FileStack(app.staged, 40.dp)
            Spacer(Modifier.width(NhakoSpace.s4))
            BasicText("${app.stagedSummary} · ${formatBytes(app.staged.sumOf { it.size })}", style = NhakoType.callout.style(c.textPrimary).copy(fontWeight = FontWeight(500)), maxLines = 1, overflow = TextOverflow.Ellipsis)
        }
    }
    var i = 0
    for ((label, list) in listOf(t("send.choose.trusted") to trusted, t("send.choose.nearby") to nearby)) {
        if (list.isEmpty()) continue
        BasicText(label, style = NhakoType.footnote.style(c.textSecondary).copy(fontWeight = FontWeight(600)))
        Column {
            for (d in list) DeviceRow(app, d, i++)
        }
    }
    NButton(t("send.choose.useCode"), { app.sheet = Sheet.ShowCode }, variant = ButtonVariant.Plain, icon = "qr-code", full = true)
}

@Composable
private fun DeviceRow(app: AppState, d: Device, order: Int) {
    val c = Nhako.colors
    val enter = remember { Animatable(0f) }
    LaunchedEffect(Unit) {
        delay((order.coerceAtMost(5) * NhakoMotion.staggerStepMs).toLong())
        enter.animateTo(1f, NhakoMotion.smooth.spec())
    }
    Row(
        Modifier.fillMaxWidth().graphicsLayer { translationY = (1 - enter.value) * 8.dp.toPx(); alpha = enter.value.coerceIn(0f, 1f) }
            .clip(RoundedCornerShape(NhakoRadius.md)).clickable(enabled = d.reachable || d.wakeable, role = Role.Button, onClick = withTap { app.sendTo(d.id) })
            .alpha(if (d.reachable || d.wakeable) 1f else 0.5f).padding(vertical = NhakoSpace.s2),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        DeviceOrb(d, 40.dp, showName = false)
        Spacer(Modifier.width(NhakoSpace.s3))
        BasicText(d.name, Modifier.weight(1f), style = NhakoType.body.style(c.textPrimary).copy(fontWeight = FontWeight(500)), maxLines = 1, overflow = TextOverflow.Ellipsis)
        d.route?.let { RouteBadge(it) } ?: BasicText(t(if (d.wakeable) "home.orb.asleep" else "home.orb.offline"), style = NhakoType.footnote.style(c.textTertiary))
        if (!d.trusted && d.route == io.github.kimzam30.nhakobeam.model.Route.Nearby) {
            Spacer(Modifier.width(NhakoSpace.s2))
            NButton(t("send.choose.trust"), { app.askTrust(d.id) }, variant = ButtonVariant.Plain, size = ButtonSize.Sm, icon = "handshake")
        }
    }
}

/** Type or paste text, then pick a device (or send it straight to the one it was opened for). */
@Composable
private fun ComposeSheet(app: AppState, sheet: Sheet.Compose, visible: Boolean, close: () -> Unit) {
    val c = Nhako.colors
    var draft by remember(sheet) { mutableStateOf(sheet.text ?: app.stagedText.orEmpty()) }
    val over = (draft.toByteArray().size - io.github.kimzam30.nhakobeam.model.TEXT_MAX).coerceAtLeast(0)
    val to = sheet.deviceId?.let { app.device(it) }
    val focus = remember { androidx.compose.ui.focus.FocusRequester() }
    LaunchedEffect(sheet) { runCatching { focus.requestFocus() } }
    fun done() {
        if (draft.isBlank() || over > 0) return
        if (to != null) app.sendTextTo(to.id, draft) else if (app.stageText(draft)) app.sheet = Sheet.Send
    }
    NSheet(visible, to?.let { t("compose.sendTo", "device" to it.name) } ?: t("compose.title"), close, footer = {
        Row(horizontalArrangement = Arrangement.spacedBy(NhakoSpace.s3)) {
            NButton(t("compose.paste"), {
                val clip = app.platform.readClipboard()
                if (clip.isEmpty()) app.toast(t("send.clipboard.empty"), io.github.kimzam30.nhakobeam.state.ToastTone.Warning)
                else draft = if (draft.isBlank()) clip else draft.trimEnd('\n') + "\n" + clip
            }, variant = ButtonVariant.Secondary, icon = "clipboard-text")
            NButton(to?.let { t("compose.sendTo", "device" to it.name) } ?: t("compose.next"), ::done, Modifier.weight(1f), full = true, enabled = draft.isNotBlank() && over == 0, icon = "paper-plane-tilt", iconWeight = IconWeight.Fill)
        }
    }) {
        NTextArea(draft, { draft = it }, Modifier.fillMaxWidth().focusRequester(focus), placeholder = t("compose.placeholder"), error = over > 0)
        if (over > 0) Row(verticalAlignment = Alignment.Top) {
            PhIcon("warning-circle", size = 16.dp, tint = c.danger, weight = IconWeight.Bold)
            Spacer(Modifier.width(NhakoSpace.s2))
            BasicText(t("compose.tooLong", "over" to formatBytes(over.toLong())), style = NhakoType.footnote.style(c.danger))
        }
    }
}

/** A device's own sheet: what it is, send something, and trust it when it's nearby and not yours. */
@Composable
private fun DeviceContent(app: AppState, d: Device) {
    val c = Nhako.colors
    Row(verticalAlignment = Alignment.CenterVertically) {
        DeviceOrb(d, 64.dp, showName = false)
        Spacer(Modifier.width(NhakoSpace.s4))
        Column(verticalArrangement = Arrangement.spacedBy(NhakoSpace.s2)) {
            BasicText(t(if (d.trusted) "device.yours" else "device.notYours"), style = NhakoType.footnote.style(c.textSecondary))
            Row(verticalAlignment = Alignment.CenterVertically) {
                d.route?.let { RouteBadge(it) }
                Spacer(Modifier.width(NhakoSpace.s3))
                PhIcon("fingerprint-simple", size = 16.dp, tint = c.textSecondary)
                Spacer(Modifier.width(NhakoSpace.s1))
                BasicText(d.fingerprint, style = NhakoType.footnote.style(c.textSecondary).copy(fontFamily = androidx.compose.ui.text.font.FontFamily.Monospace))
            }
        }
    }
    val can = d.reachable || d.wakeable
    listOf(Triple("files", "files", t("device.send.files")), Triple("folder", "folder-simple", t("device.send.folder")), Triple("text", "text-aa", t("device.send.text"))).forEachIndexed { i, (kind, icon, label) ->
        val enter = remember { Animatable(0f) }
        LaunchedEffect(Unit) {
            delay((i * NhakoMotion.staggerStepMs).toLong())
            enter.animateTo(1f, NhakoMotion.smooth.spec())
        }
        Row(
            Modifier.fillMaxWidth().graphicsLayer { translationY = (1 - enter.value) * 8.dp.toPx(); alpha = enter.value.coerceIn(0f, 1f) }
                .clip(RoundedCornerShape(NhakoRadius.md)).clickable(enabled = can, role = Role.Button, onClick = withTap {
                    when (kind) {
                        "text" -> app.sheet = Sheet.Compose(deviceId = d.id)
                        "folder" -> {
                            app.sheet = null
                            app.pickTarget = d.id
                            app.platform.pickFolder()
                        }
                        else -> {
                            app.sheet = null
                            if (app.hasStaged) app.sendTo(d.id) else {
                                app.pickTarget = d.id
                                app.platform.pickFiles()
                            }
                        }
                    }
                }).alpha(if (can) 1f else 0.5f).padding(vertical = NhakoSpace.s2),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Box(Modifier.size(40.dp).clip(CircleShape).background(c.fill), contentAlignment = Alignment.Center) { PhIcon(icon, size = 22.dp, tint = c.accentText) }
            Spacer(Modifier.width(NhakoSpace.s3))
            BasicText(label, Modifier.weight(1f), style = NhakoType.body.style(c.textPrimary).copy(fontWeight = FontWeight(500)))
            PhIcon("caret-right", size = 16.dp, tint = c.textTertiary, weight = IconWeight.Bold)
        }
    }
    if (!d.trusted && d.reachable && d.route == io.github.kimzam30.nhakobeam.model.Route.Nearby) {
        Column(
            Modifier.fillMaxWidth().clip(RoundedCornerShape(NhakoRadius.md)).background(c.surfaceSunken).padding(NhakoSpace.s4),
            verticalArrangement = Arrangement.spacedBy(NhakoSpace.s3),
        ) {
            BasicText(t("device.trust.hint"), style = NhakoType.footnote.style(c.textSecondary))
            NButton(t("device.trust"), { app.askTrust(d.id) }, full = true, icon = "handshake")
        }
    }
}

/** You and the other device, with a line between that charges while the ask is out. */
@Composable
private fun TrustPair(app: AppState, deviceId: String, live: Boolean, waiting: Boolean) {
    val c = Nhako.colors
    val reduced = Nhako.reducedMotion
    val run = androidx.compose.animation.core.rememberInfiniteTransition(label = "charge")
    val p by run.animateFloat(0f, 1f, androidx.compose.animation.core.infiniteRepeatable(tween(1100, easing = NhakoMotion.Easing.standard)), label = "run")
    Row(Modifier.fillMaxWidth().padding(vertical = NhakoSpace.s4), horizontalArrangement = Arrangement.Center, verticalAlignment = Alignment.CenterVertically) {
        DeviceOrb(null, 64.dp, self = true, selfName = app.selfName, showName = false)
        Box(
            Modifier.padding(horizontal = NhakoSpace.s2).size(72.dp, 2.dp).clip(CircleShape).background(c.beamTrack).drawWithContent {
                if (!live) return@drawWithContent
                if (reduced) drawRect(io.github.kimzam30.nhakobeam.design.NhakoBrand.yellow, alpha = 0.5f)
                else {
                    val w = size.width * 0.4f
                    drawRect(io.github.kimzam30.nhakobeam.design.NhakoBrand.yellow, Offset(-w + (size.width + w) * p, 0f), Size(w, size.height))
                }
            },
        )
        app.device(deviceId)?.let { DeviceOrb(it, 64.dp, showName = false, waiting = waiting) }
    }
}

/** The value both screens show, read off by a thin line once (like the paired sheet's). */
@Composable
private fun CheckChip(key: Any, label: String, check: String) {
    val c = Nhako.colors
    val reduced = Nhako.reducedMotion
    val scan = remember(key, check) { Animatable(0f) }
    LaunchedEffect(key, check) {
        delay(260)
        scan.animateTo(1f, tween(if (reduced) NhakoMotion.Duration.base else 640, easing = NhakoMotion.Easing.standard))
    }
    Column(Modifier.fillMaxWidth(), horizontalAlignment = Alignment.CenterHorizontally) {
        BasicText(label, style = NhakoType.footnote.style(c.textSecondary).copy(textAlign = androidx.compose.ui.text.style.TextAlign.Center))
        Spacer(Modifier.height(NhakoSpace.s2))
        Row(
            Modifier.clip(RoundedCornerShape(NhakoRadius.sm)).background(c.fill)
                .drawWithContent {
                    drawContent()
                    val v = scan.value
                    if (v >= 1f) return@drawWithContent
                    if (reduced) drawRect(c.fill, alpha = 1f - v)
                    else {
                        val x = size.width * v
                        drawRect(c.fill, Offset(x, 0f), Size(size.width - x, size.height))
                        drawRect(c.accent, Offset(x, 0f), Size(2.dp.toPx(), size.height))
                    }
                }
                .padding(horizontal = NhakoSpace.s4, vertical = NhakoSpace.s2),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            PhIcon("fingerprint-simple", size = 18.dp, tint = c.textPrimary)
            Spacer(Modifier.width(NhakoSpace.s2))
            BasicText(check, style = NhakoType.headline.style(c.textPrimary))
        }
    }
}

@Composable
private fun ShowCodeContent(app: AppState) {
    LaunchedEffect(Unit) { if (app.code == null) app.newCode() }
    BasicText(t("code.show.body"), style = NhakoType.body.style(Nhako.colors.textSecondary))
    val code = app.code
    if (code != null) CodeDisplay(app, code)
    else Box(Modifier.fillMaxWidth().height(232.dp), contentAlignment = Alignment.Center) { Spinner(Nhako.colors.accent, 28.dp) }
}

@Composable
private fun EnterCodeSheet(app: AppState, visible: Boolean, close: () -> Unit) {
    val c = Nhako.colors
    val view = LocalView.current
    val scope = rememberCoroutineScope()
    val state = remember { CodeInputState() }
    var checking by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf("") }

    fun redeem() {
        if (checking || !state.complete) return
        checking = true
        error = ""
        scope.launch {
            val r = runCatching { app.engine.redeemCode(state.words.toList()) }.getOrElse {
                checking = false
                error = it.message ?: t("code.enter.wrong")
                return@launch
            }
            checking = false
            if (r.ok) return@launch // the 'paired' event opens the paired sheet
            NhakoHaptic.Decline.perform(view)
            app.sound(NhakoSound.Error)
            when (r.reason) {
                "unknownWord" -> {
                    error = t("code.enter.unknownWord")
                    state.shake(r.index)
                }
                "tokenRejected" -> {
                    error = t("code.enter.tokenRejected")
                    state.shake(null)
                }
                else -> {
                    error = t("code.enter.wrong")
                    state.shake(null)
                }
            }
        }
    }

    NSheet(visible, t("code.enter.title"), close, footer = {
        Row(horizontalArrangement = Arrangement.spacedBy(NhakoSpace.s3)) {
            NButton(t("code.enter.scan"), { app.sheet = Sheet.Scan }, variant = ButtonVariant.Secondary, icon = "scan")
            NButton(t("code.enter.connect"), ::redeem, Modifier.weight(1f), full = true, loading = checking, enabled = state.complete)
        }
    }) {
        BasicText(t("code.enter.body"), style = NhakoType.body.style(c.textSecondary))
        CodeInput(state, enabled = !checking, onSubmit = ::redeem)
        if (error.isNotEmpty()) {
            Row(verticalAlignment = Alignment.Top) {
                PhIcon("warning-circle", size = 16.dp, tint = c.danger, weight = IconWeight.Bold)
                Spacer(Modifier.width(NhakoSpace.s2))
                BasicText(error, style = NhakoType.footnote.style(c.danger))
            }
        }
    }
}

@Composable
private fun ScanContent(app: AppState) {
    val scope = rememberCoroutineScope()
    var connecting by remember { mutableStateOf(false) }
    BasicText(t("code.enter.body"), style = NhakoType.body.style(Nhako.colors.textSecondary))
    QrScanner(onFound = { words ->
        if (connecting) return@QrScanner
        connecting = true
        scope.launch {
            val r = runCatching { app.engine.redeemCode(words) }.getOrNull()
            connecting = false
            if (r?.ok != true) {
                app.toast(if (r?.reason == "tokenRejected") t("code.enter.tokenRejected") else t("code.enter.wrong"), io.github.kimzam30.nhakobeam.state.ToastTone.Danger)
                app.sheet = Sheet.EnterCode
            }
        }
    })
    if (connecting) Row(verticalAlignment = Alignment.CenterVertically) {
        Spinner(Nhako.colors.accent)
        Spacer(Modifier.width(NhakoSpace.s3))
        BasicText(t("transfer.connecting"), style = NhakoType.callout.style(Nhako.colors.textSecondary))
    }
}

/** The two devices slide together and the check value both screens show appears. */
@Composable
private fun PairedSheet(app: AppState, sheet: Sheet.Paired, visible: Boolean) {
    val c = Nhako.colors
    val device = app.device(sheet.deviceId) ?: return
    var trust by remember(sheet) { mutableStateOf(true) }
    val gap = remember(sheet) { Animatable(70f) }
    var met by remember(sheet) { mutableStateOf(false) }
    val meetBolt = remember(sheet) { Animatable(0.3f) }
    var zap by remember(sheet) { mutableIntStateOf(0) }
    val reduced = Nhako.reducedMotion
    LaunchedEffect(sheet) {
        delay(120)
        gap.animateTo(6f, NhakoMotion.smooth.spec())
        app.bump(device.id)
        // Where they meet, a bolt joins them.
        met = true
        delay(60)
        zap++
        if (reduced) meetBolt.snapTo(1f) else meetBolt.animateTo(1f, NhakoMotion.bouncy.spec())
    }
    // The check code is read off left to right by a thin line, once.
    val scan = remember(sheet) { Animatable(0f) }
    LaunchedEffect(sheet) {
        delay(260)
        scan.animateTo(1f, tween(if (reduced) NhakoMotion.Duration.base else 640, easing = NhakoMotion.Easing.standard))
    }
    val nearby = sheet.via == io.github.kimzam30.nhakobeam.model.PairVia.Nearby
    fun finish(send: Boolean) {
        // From Nearby, both have already saved each other.
        if (trust && !nearby) app.engine.setTrusted(device.id, true)
        app.sheet = null
        if (send && app.staged.isNotEmpty()) app.sendTo(device.id)
    }
    NSheet(visible, t(if (nearby) "code.matched.nearby" else "code.matched", "device" to device.name), { finish(false) }, footer = {
        if (app.staged.isNotEmpty()) NButton(t("code.matched.send", "summary" to app.stagedSummary), { finish(true) }, size = ButtonSize.Lg, full = true, icon = "paper-plane-tilt", iconWeight = IconWeight.Fill)
        else NButton(t("code.matched.continue"), { finish(false) }, size = ButtonSize.Lg, full = true)
    }) {
        Box(Modifier.fillMaxWidth(), contentAlignment = Alignment.Center) {
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.Center) {
                Box(Modifier.graphicsLayer { translationX = -gap.value * density }) { DeviceOrb(null, 72.dp, self = true, selfName = app.selfName, showName = false) }
                Box(Modifier.graphicsLayer { translationX = gap.value * density }) { DeviceOrb(device, 72.dp, showName = false, pulse = app.pulse[device.id] ?: 0) }
            }
            if (met) Bolt(30.dp, Modifier.offset(y = (-8).dp).sparks(zap, count = 6, reach = 34f).graphicsLayer { scaleX = meetBolt.value; scaleY = meetBolt.value; alpha = meetBolt.value.coerceIn(0f, 1f) })
        }
        Column(Modifier.fillMaxWidth(), horizontalAlignment = Alignment.CenterHorizontally) {
            BasicText(t("code.matched.verify"), style = NhakoType.footnote.style(c.textSecondary))
            Spacer(Modifier.height(NhakoSpace.s2))
            Row(
                Modifier.clip(RoundedCornerShape(NhakoRadius.sm)).background(c.fill)
                    .drawWithContent {
                        drawContent()
                        // A cover in the chip's own color slides off to the right, led by an accent line.
                        val p = scan.value
                        if (p >= 1f) return@drawWithContent
                        if (reduced) {
                            drawRect(c.fill, alpha = 1f - p)
                        } else {
                            val x = size.width * p
                            drawRect(c.fill, Offset(x, 0f), Size(size.width - x, size.height))
                            drawRect(c.accent, Offset(x, 0f), Size(2.dp.toPx(), size.height))
                        }
                    }
                    .padding(horizontal = NhakoSpace.s4, vertical = NhakoSpace.s2),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                PhIcon("fingerprint-simple", size = 18.dp, tint = c.textPrimary)
                Spacer(Modifier.width(NhakoSpace.s2))
                BasicText(sheet.check, style = NhakoType.headline.style(c.textPrimary))
            }
        }
        if (nearby) BasicText(t("code.matched.nearby.body"), Modifier.fillMaxWidth(), style = NhakoType.body.style(c.textSecondary).copy(textAlign = androidx.compose.ui.text.style.TextAlign.Center))
        else NToggle(trust, t("code.matched.trust"), { trust = it }, hint = t("code.matched.trust.body"))
    }
}

@Composable
private fun DetailsContent(app: AppState, id: String) {
    val c = Nhako.colors
    val tr = app.transfers.find { it.id == id } ?: return
    var shown by remember(id) { mutableIntStateOf(FILES_PAGE) }
    for (f in tr.files.take(shown)) {
        val done = tr.fileBytes[f.id] ?: 0
        val p = percent(done, f.size)
        Row(verticalAlignment = Alignment.CenterVertically) {
            FileName(f, Modifier.weight(1f))
            BasicText(if (p == 100) formatBytes(f.size) else "$p%", style = NhakoType.footnote.style(c.textSecondary))
            Spacer(Modifier.width(NhakoSpace.s3))
            if (p == 100) PhIcon("check-circle", size = 18.dp, tint = c.success, weight = IconWeight.Fill)
            else {
                val w by animateFloatAsState(p / 100f, NhakoMotion.smooth.spec(), label = "file")
                Box(Modifier.size(48.dp, 4.dp).clip(CircleShape).background(c.beamTrack)) {
                    Box(Modifier.fillMaxWidth(w).height(4.dp).background(c.accent))
                }
            }
        }
    }
    MoreFiles(tr.files.size, shown) { shown += FILES_PAGE }
    BasicText("${summarizeFiles(tr.files)} · ${formatBytes(tr.totalBytes)}", style = NhakoType.footnote.style(c.textSecondary))
}

/** A file's name, after the folder it's in (lighter) when it came inside a sent folder. */
@Composable
internal fun FileName(f: io.github.kimzam30.nhakobeam.model.FileItem, modifier: Modifier = Modifier) {
    val c = Nhako.colors
    val text = androidx.compose.ui.text.buildAnnotatedString {
        f.folder?.let {
            pushStyle(androidx.compose.ui.text.SpanStyle(color = c.textTertiary))
            append(middleTruncate(it, 24) + "/")
            pop()
        }
        append(middleTruncate(f.name, 34))
    }
    BasicText(text, modifier, style = NhakoType.callout.style(c.textPrimary), maxLines = 1, overflow = TextOverflow.Ellipsis)
}

/** Why opening a received program asks first: a warning that pops in, then the reason. */
@Composable
private fun RiskyOpen(sheet: Sheet.ConfirmOpen) {
    val c = Nhako.colors
    val reduced = Nhako.reducedMotion
    val pop = remember(sheet) { Animatable(0f) }
    LaunchedEffect(sheet) {
        delay(80)
        pop.animateTo(1f, (if (reduced) NhakoMotion.smooth else NhakoMotion.bouncy).spec())
    }
    Row(verticalAlignment = Alignment.Top) {
        Box(
            Modifier.size(48.dp).graphicsLayer {
                alpha = pop.value.coerceIn(0f, 1f)
                if (!reduced) {
                    scaleX = 0.4f + 0.6f * pop.value
                    scaleY = 0.4f + 0.6f * pop.value
                }
            }.clip(CircleShape).background(c.warningSubtle),
            contentAlignment = Alignment.Center,
        ) { PhIcon("warning-circle", size = 28.dp, tint = c.warning, weight = IconWeight.Fill) }
        Spacer(Modifier.width(NhakoSpace.s4))
        BasicText(t("open.risky.body", "device" to sheet.deviceName), style = NhakoType.body.style(c.textSecondary))
    }
}
