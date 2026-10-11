// Every bottom sheet: choose a device, show a code, enter a code, scan, paired, transfer files,
// remove a trusted device. The Compose twin of desktop/src/components/Sheets.svelte.
package io.github.kimzam30.nhakobeam.ui.screens

import androidx.activity.compose.BackHandler
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
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
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import io.github.kimzam30.nhakobeam.design.NhakoHaptic
import io.github.kimzam30.nhakobeam.design.NhakoMotion
import io.github.kimzam30.nhakobeam.design.NhakoRadius
import io.github.kimzam30.nhakobeam.design.NhakoSpace
import io.github.kimzam30.nhakobeam.design.NhakoType
import io.github.kimzam30.nhakobeam.model.Device
import io.github.kimzam30.nhakobeam.state.AppState
import io.github.kimzam30.nhakobeam.state.Sheet
import io.github.kimzam30.nhakobeam.ui.Copy.t
import io.github.kimzam30.nhakobeam.ui.components.ButtonSize
import io.github.kimzam30.nhakobeam.ui.components.ButtonVariant
import io.github.kimzam30.nhakobeam.ui.components.CodeDisplay
import io.github.kimzam30.nhakobeam.ui.components.CodeInput
import io.github.kimzam30.nhakobeam.ui.components.CodeInputState
import io.github.kimzam30.nhakobeam.ui.components.DeviceOrb
import io.github.kimzam30.nhakobeam.ui.components.FileStack
import io.github.kimzam30.nhakobeam.ui.components.NButton
import io.github.kimzam30.nhakobeam.ui.components.NSheet
import io.github.kimzam30.nhakobeam.ui.components.NToggle
import io.github.kimzam30.nhakobeam.ui.components.QrScanner
import io.github.kimzam30.nhakobeam.ui.components.RouteBadge
import io.github.kimzam30.nhakobeam.ui.components.Spinner
import io.github.kimzam30.nhakobeam.ui.formatBytes
import io.github.kimzam30.nhakobeam.ui.middleTruncate
import io.github.kimzam30.nhakobeam.ui.percent
import io.github.kimzam30.nhakobeam.ui.summarizeFiles
import io.github.kimzam30.nhakobeam.ui.theme.IconWeight
import io.github.kimzam30.nhakobeam.ui.theme.Nhako
import io.github.kimzam30.nhakobeam.ui.theme.PhIcon
import io.github.kimzam30.nhakobeam.ui.theme.style
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
    }
}

@Composable
private fun SendContent(app: AppState) {
    val c = Nhako.colors
    val trusted = app.devices.filter { it.trusted }.sortedByDescending { it.reachable }
    val nearby = app.devices.filter { !it.trusted && it.reachable }
    if (app.staged.isNotEmpty()) {
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
            .clip(RoundedCornerShape(NhakoRadius.md)).clickable(enabled = d.reachable, role = Role.Button) { app.sendTo(d.id) }
            .alpha(if (d.reachable) 1f else 0.5f).padding(vertical = NhakoSpace.s2),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        DeviceOrb(d, 40.dp, showName = false)
        Spacer(Modifier.width(NhakoSpace.s3))
        BasicText(d.name, Modifier.weight(1f), style = NhakoType.body.style(c.textPrimary).copy(fontWeight = FontWeight(500)), maxLines = 1, overflow = TextOverflow.Ellipsis)
        d.route?.let { RouteBadge(it) } ?: BasicText(t("home.orb.offline"), style = NhakoType.footnote.style(c.textTertiary))
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
    LaunchedEffect(sheet) {
        delay(120)
        gap.animateTo(6f, NhakoMotion.smooth.spec())
        app.bump(device.id)
    }
    fun finish(send: Boolean) {
        if (trust) app.engine.setTrusted(device.id, true)
        app.sheet = null
        if (send && app.staged.isNotEmpty()) app.sendTo(device.id)
    }
    NSheet(visible, t("code.matched", "device" to device.name), { finish(false) }, footer = {
        if (app.staged.isNotEmpty()) NButton(t("code.matched.send", "summary" to app.stagedSummary), { finish(true) }, size = ButtonSize.Lg, full = true, icon = "paper-plane-tilt", iconWeight = IconWeight.Fill)
        else NButton(t("code.matched.continue"), { finish(false) }, size = ButtonSize.Lg, full = true)
    }) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.Center) {
            Box(Modifier.graphicsLayer { translationX = -gap.value * density }) { DeviceOrb(null, 72.dp, self = true, selfName = app.selfName, showName = false) }
            Box(Modifier.graphicsLayer { translationX = gap.value * density }) { DeviceOrb(device, 72.dp, showName = false, pulse = app.pulse[device.id] ?: 0) }
        }
        Column(Modifier.fillMaxWidth(), horizontalAlignment = Alignment.CenterHorizontally) {
            BasicText(t("code.matched.verify"), style = NhakoType.footnote.style(c.textSecondary))
            Spacer(Modifier.height(NhakoSpace.s2))
            Row(Modifier.clip(RoundedCornerShape(NhakoRadius.sm)).background(c.fill).padding(horizontal = NhakoSpace.s4, vertical = NhakoSpace.s2), verticalAlignment = Alignment.CenterVertically) {
                PhIcon("fingerprint-simple", size = 18.dp, tint = c.textPrimary)
                Spacer(Modifier.width(NhakoSpace.s2))
                BasicText(sheet.check, style = NhakoType.headline.style(c.textPrimary))
            }
        }
        NToggle(trust, t("code.matched.trust"), { trust = it }, hint = t("code.matched.trust.body"))
    }
}

@Composable
private fun DetailsContent(app: AppState, id: String) {
    val c = Nhako.colors
    val tr = app.transfers.find { it.id == id } ?: return
    for (f in tr.files) {
        val done = tr.fileBytes[f.id] ?: 0
        val p = percent(done, f.size)
        Row(verticalAlignment = Alignment.CenterVertically) {
            BasicText(middleTruncate(f.name, 34), Modifier.weight(1f), style = NhakoType.callout.style(c.textPrimary), maxLines = 1)
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
    BasicText("${summarizeFiles(tr.files)} · ${formatBytes(tr.totalBytes)}", style = NhakoType.footnote.style(c.textSecondary))
}
