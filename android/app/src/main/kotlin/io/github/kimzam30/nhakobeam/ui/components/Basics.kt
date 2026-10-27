// Buttons, switches, segmented controls, banners and fields, with the press springs from the
// motion tokens. The Compose twins of desktop/src/components/Button.svelte, Toggle.svelte, etc.
package io.github.kimzam30.nhakobeam.ui.components

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsFocusedAsState
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.toggleable
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.text.TextAutoSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawWithContent
import androidx.compose.ui.draw.scale
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import io.github.kimzam30.nhakobeam.design.NhakoBrand
import io.github.kimzam30.nhakobeam.design.NhakoHaptic
import io.github.kimzam30.nhakobeam.design.NhakoMotion
import io.github.kimzam30.nhakobeam.design.NhakoRadius
import io.github.kimzam30.nhakobeam.design.NhakoSound
import io.github.kimzam30.nhakobeam.design.NhakoSpace
import io.github.kimzam30.nhakobeam.design.NhakoType
import io.github.kimzam30.nhakobeam.ui.Copy.t
import io.github.kimzam30.nhakobeam.ui.LocalSound
import io.github.kimzam30.nhakobeam.ui.theme.IconWeight
import io.github.kimzam30.nhakobeam.ui.theme.Nhako
import io.github.kimzam30.nhakobeam.ui.theme.PhIcon
import io.github.kimzam30.nhakobeam.ui.theme.style
import io.github.kimzam30.nhakobeam.ui.withTap
import kotlinx.coroutines.launch

/** Scales down a touch while pressed (press spring), like every tappable thing in the app. */
@Composable
fun Modifier.pressScale(interaction: MutableInteractionSource, scale: Float = NhakoMotion.Scale.pressed): Modifier {
    val pressed by interaction.collectIsPressedAsState()
    val s by animateFloatAsState(if (pressed && !Nhako.reducedMotion) scale else 1f, NhakoMotion.press.spec(), label = "press")
    return this.scale(s)
}

enum class ButtonVariant { Primary, Secondary, Plain, Danger }
enum class ButtonSize(val height: Dp, val padding: Dp, val icon: Dp) { Sm(36.dp, 14.dp, 16.dp), Md(44.dp, 18.dp, 18.dp), Lg(52.dp, 22.dp, 20.dp) }

@Composable
fun NButton(
    text: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    variant: ButtonVariant = ButtonVariant.Primary,
    size: ButtonSize = ButtonSize.Md,
    icon: String? = null,
    iconWeight: IconWeight = IconWeight.Duotone,
    enabled: Boolean = true,
    loading: Boolean = false,
    full: Boolean = false,
    padding: Dp = size.padding,
    /** Its sound when pressed (desktop: the `sound` prop of Button.svelte). */
    sound: NhakoSound = NhakoSound.Tap,
    /** A primary button throws sparks when pressed; off where the screen zaps on its own. */
    sparks: Boolean = true,
) {
    val c = Nhako.colors
    val interaction = remember { MutableInteractionSource() }
    val play = LocalSound.current
    // Release: a thin yellow rim flashes; a primary action also throws a few sparks.
    val rim = remember { Animatable(0f) }
    var fire by remember { mutableIntStateOf(0) }
    val scope = rememberCoroutineScope()
    val (bg, fg) = when (variant) {
        ButtonVariant.Primary -> c.accent to c.onAccent
        ButtonVariant.Secondary -> c.fill to c.textPrimary
        ButtonVariant.Plain -> Color.Transparent to c.accentText
        ButtonVariant.Danger -> c.danger to c.onDanger
    }
    val active = enabled && !loading
    Row(
        modifier = modifier
            .then(if (full) Modifier.fillMaxWidth() else Modifier)
            .heightIn(min = size.height)
            .sparks(if (variant == ButtonVariant.Primary && sparks) fire else 0, count = 4, reach = 28f)
            .pressScale(interaction)
            .alpha(if (enabled) 1f else 0.45f)
            .rimFlash(rim.value)
            .clip(RoundedCornerShape(NhakoRadius.full))
            .background(bg)
            .clickable(interaction, indication = null, enabled = active, role = Role.Button) {
                play(sound)
                fire++
                scope.launch {
                    rim.snapTo(0.9f)
                    rim.animateTo(0f, tween(NhakoMotion.Bolt.flashMs, easing = NhakoMotion.Easing.enter))
                }
                onClick()
            }
            .padding(horizontal = padding),
        horizontalArrangement = Arrangement.Center,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        if (loading) {
            Spinner(color = fg, size = size.icon)
            Spacer(Modifier.width(NhakoSpace.s3))
        } else if (icon != null) {
            PhIcon(icon, size = size.icon, tint = fg, weight = iconWeight)
            Spacer(Modifier.width(NhakoSpace.s3))
        }
        val type = when (size) { ButtonSize.Sm -> NhakoType.callout; ButtonSize.Md -> NhakoType.body; ButtonSize.Lg -> NhakoType.headline }
        // A label that does not fit shrinks a little before it gets cut (narrow phones, large font).
        BasicText(
            text,
            style = type.style(fg).copy(fontWeight = androidx.compose.ui.text.font.FontWeight(600)),
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            autoSize = TextAutoSize.StepBased(minFontSize = 12.sp, maxFontSize = type.size, stepSize = 0.5.sp),
        )
    }
}

/** The thin yellow rim around a pill, at [alpha] (0 = none). Alpha only, so reduced motion keeps it. */
fun Modifier.rimFlash(alpha: Float): Modifier = drawWithContent {
    drawContent()
    if (alpha <= 0f) return@drawWithContent
    val o = 3.dp.toPx()
    drawRoundRect(
        NhakoBrand.yellow, topLeft = Offset(-o, -o), size = Size(size.width + 2 * o, size.height + 2 * o),
        cornerRadius = CornerRadius(size.height / 2 + o), style = Stroke(2.dp.toPx()), alpha = alpha,
    )
}

/**
 * How each icon answers a press (desktop: `nudges` in IconButton.svelte): the gear turns, the bin
 * lid lifts, the x spins a quarter, the back caret leans back. [p] runs 0 to 1. Others pop.
 */
private fun Modifier.nudge(icon: String, p: Float): Modifier = graphicsLayer {
    if (p <= 0f || p >= 1f) return@graphicsLayer
    val bump = if (p < 0.4f) p / 0.4f else 1f - (p - 0.4f) / 0.6f
    when (icon) {
        "gear-six" -> rotationZ = 60f * NhakoMotion.Easing.standard.transform(p)
        "x" -> rotationZ = 90f * NhakoMotion.Easing.standard.transform(p)
        "trash" -> { translationY = -3.dp.toPx() * bump; rotationZ = -10f * bump }
        "caret-left" -> translationX = -4.dp.toPx() * bump
        "clock-counter-clockwise" -> rotationZ = -40f * bump
        "files" -> translationY = -3.dp.toPx() * bump
        else -> { val s = if (p < 0.55f) 0.7f + 0.42f * (p / 0.55f) else 1.12f - 0.12f * ((p - 0.55f) / 0.45f); scaleX = s; scaleY = s }
    }
}

@Composable
fun NIconButton(icon: String, label: String, onClick: () -> Unit, modifier: Modifier = Modifier, tint: Color = Nhako.colors.textSecondary, size: Dp = 22.dp, weight: IconWeight = IconWeight.Duotone, enabled: Boolean = true, sound: NhakoSound = NhakoSound.Tap) {
    val interaction = remember { MutableInteractionSource() }
    val play = LocalSound.current
    val reduced = Nhako.reducedMotion
    val nudge = remember { Animatable(1f) }
    val scope = rememberCoroutineScope()
    Box(
        modifier
            .size(48.dp)
            .pressScale(interaction, 0.9f)
            .clip(CircleShape)
            .clickable(interaction, indication = null, enabled = enabled, role = Role.Button) {
                play(sound)
                if (!reduced) scope.launch {
                    nudge.snapTo(0f)
                    nudge.animateTo(1f, tween(380, easing = LinearEasing))
                }
                onClick()
            }
            .semantics { contentDescription = label },
        contentAlignment = Alignment.Center,
    ) {
        PhIcon(icon, Modifier.nudge(icon, nudge.value), size = size, tint = if (enabled) tint else tint.copy(alpha = 0.4f), weight = weight)
    }
}

/** A small spinning arc (loading). Reduced motion shows it still. */
@Composable
fun Spinner(color: Color, size: Dp = 18.dp) {
    val reduced = Nhako.reducedMotion
    val turn = rememberInfiniteTransition(label = "spin").animateFloat(0f, 360f, infiniteRepeatable(tween(900, easing = LinearEasing)), label = "turn")
    Canvas(Modifier.size(size)) {
        drawArc(color, if (reduced) 0f else turn.value, 270f, false, style = Stroke(width = 2.dp.toPx(), cap = StrokeCap.Round))
    }
}

/** On/off switch with a label and an optional hint underneath; the thumb moves on the snappy spring. */
@Composable
fun NToggle(checked: Boolean, label: String, onChange: (Boolean) -> Unit, modifier: Modifier = Modifier, hint: String? = null) {
    val c = Nhako.colors
    val view = LocalView.current
    val play = LocalSound.current
    val x by animateDpAsState(if (checked) 22.dp else 2.dp, NhakoMotion.snappy.spec(), label = "thumb")
    // The thumb carries a small bolt: lit (yellow, a little bigger) when on, dim when off.
    val lit by animateFloatAsState(if (checked) 1f else 0.85f, if (Nhako.reducedMotion) tween(0) else NhakoMotion.bouncy.spec(), label = "lit")
    var zap by remember { mutableIntStateOf(0) }
    val track by animateColorAsState(if (checked) c.accent else c.fillPressed, tween(NhakoMotion.Duration.fast), label = "track")
    Row(
        modifier
            .fillMaxWidth()
            .heightIn(min = 52.dp)
            .toggleable(checked, role = Role.Switch) {
                (if (it) NhakoHaptic.ToggleOn else NhakoHaptic.ToggleOff).perform(view)
                if (it) zap++
                onChange(it)
                // After the change, so turning Sounds itself off is silent.
                play(if (it) NhakoSound.ToggleOn else NhakoSound.ToggleOff)
            },
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(Modifier.weight(1f).padding(vertical = NhakoSpace.s3)) {
            BasicText(label, style = NhakoType.body.style(c.textPrimary))
            if (hint != null) BasicText(hint, style = NhakoType.footnote.style(c.textSecondary))
        }
        Spacer(Modifier.width(NhakoSpace.s5))
        Box(Modifier.size(48.dp, 28.dp).clip(RoundedCornerShape(NhakoRadius.full)).background(track)) {
            Box(
                Modifier.offset { IntOffset(x.roundToPx(), 2.dp.roundToPx()) }.size(24.dp).sparks(zap, count = 3, reach = 22f, sizeDp = 11f)
                    .shadow(2.dp, CircleShape).clip(CircleShape).background(Color.White),
                contentAlignment = Alignment.Center,
            ) {
                Bolt(14.dp, Modifier.graphicsLayer { scaleX = lit; scaleY = lit }, brand = checked, tint = c.textDisabled)
            }
        }
    }
}

data class SegmentOption<T>(val value: T, val label: String, val icon: String? = null)

/** A row of choices with a pill that slides to the chosen one (smooth spring). */
@Composable
fun <T> NSegmented(options: List<SegmentOption<T>>, value: T, onChange: (T) -> Unit, modifier: Modifier = Modifier) {
    val c = Nhako.colors
    val play = LocalSound.current
    val index = options.indexOfFirst { it.value == value }.coerceAtLeast(0)
    // With the system font large, labels get two lines and the icons step aside to make room.
    val large = largeText()
    BoxWithConstraints(modifier.fillMaxWidth().height(if (large) 80.dp else 40.dp).clip(RoundedCornerShape(NhakoRadius.sm)).background(c.fill).padding(3.dp)) {
        val w = maxWidth / options.size
        val h = maxHeight
        val x by animateDpAsState(w * index, NhakoMotion.smooth.spec(), label = "pill")
        Box(Modifier.offset { IntOffset(x.roundToPx(), 0) }.width(w).height(h).shadow(1.dp, RoundedCornerShape(NhakoRadius.sm - 3.dp)).clip(RoundedCornerShape(NhakoRadius.sm - 3.dp)).background(c.surface))
        Row(Modifier.fillMaxWidth()) {
            options.forEachIndexed { i, o ->
                val on = i == index
                Row(
                    Modifier.weight(1f).height(h).selectable(on, role = Role.RadioButton) {
                        play(NhakoSound.Tap)
                        onChange(o.value)
                    },
                    horizontalArrangement = Arrangement.Center,
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    if (o.icon != null && !large) {
                        // The chosen option's icon pops in (desktop: Icon.svelte's crossfade).
                        val pop = remember { Animatable(1f) }
                        val reduced = Nhako.reducedMotion
                        LaunchedEffect(on) {
                            if (on && !reduced) {
                                pop.snapTo(0.6f)
                                pop.animateTo(1f, NhakoMotion.snappy.spec())
                            }
                        }
                        PhIcon(o.icon, Modifier.graphicsLayer { scaleX = pop.value; scaleY = pop.value }, size = 16.dp, tint = if (on) c.textPrimary else c.textSecondary, weight = if (on) IconWeight.Fill else IconWeight.Duotone)
                        Spacer(Modifier.width(NhakoSpace.s2))
                    }
                    BasicText(
                        o.label,
                        Modifier.padding(horizontal = NhakoSpace.s1),
                        style = NhakoType.callout.style(if (on) c.textPrimary else c.textSecondary).copy(fontWeight = androidx.compose.ui.text.font.FontWeight(if (on) 600 else 500), textAlign = TextAlign.Center),
                        maxLines = if (large) 2 else 1,
                    )
                }
            }
        }
    }
}

/** The system font is at 150% or more: things that share a line stack instead, so nothing is cut off. */
@Composable
fun largeText(): Boolean = androidx.compose.ui.platform.LocalDensity.current.fontScale >= 1.5f

enum class BannerTone { Warning, Danger, Info }

@Composable
fun NBanner(
    text: String,
    modifier: Modifier = Modifier,
    icon: String = "warning-circle",
    tone: BannerTone = BannerTone.Warning,
    actionLabel: String? = null,
    onAction: (() -> Unit)? = null,
    /** A quieter second action, before the main one (the update's Not now). */
    otherLabel: String? = null,
    onOther: (() -> Unit)? = null,
) {
    val c = Nhako.colors
    val (bg, fg) = when (tone) {
        BannerTone.Warning -> c.warningSubtle to (if (Nhako.dark) c.warning else Color(0xFF8A5300))
        BannerTone.Danger -> c.dangerSubtle to c.danger
        BannerTone.Info -> c.accentSubtle to c.accentText
    }
    Row(
        modifier.fillMaxWidth().clip(RoundedCornerShape(NhakoRadius.md)).background(bg).padding(horizontal = NhakoSpace.s5, vertical = NhakoSpace.s4),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        PhIcon(icon, size = 20.dp, tint = fg)
        Spacer(Modifier.width(NhakoSpace.s4))
        BasicText(text, Modifier.weight(1f), style = NhakoType.callout.style(fg))
        if (otherLabel != null && onOther != null) {
            Spacer(Modifier.width(NhakoSpace.s2))
            BasicText(otherLabel, Modifier.clickable(role = Role.Button, onClick = withTap(onOther)).padding(NhakoSpace.s2), style = NhakoType.callout.style(fg).copy(fontWeight = androidx.compose.ui.text.font.FontWeight(600)))
        }
        if (actionLabel != null && onAction != null) {
            Spacer(Modifier.width(NhakoSpace.s3))
            BasicText(actionLabel, Modifier.clickable(role = Role.Button, onClick = withTap(onAction)).padding(NhakoSpace.s2), style = NhakoType.callout.style(fg).copy(fontWeight = androidx.compose.ui.text.font.FontWeight(600)))
        }
    }
}

/** A rounded white card, the settings and activity surface. */
@Composable
fun NCard(modifier: Modifier = Modifier, padding: PaddingValues = PaddingValues(horizontal = NhakoSpace.s5, vertical = NhakoSpace.s3), content: @Composable () -> Unit) {
    Box(modifier.fillMaxWidth().shadow(1.dp, RoundedCornerShape(NhakoRadius.lg)).clip(RoundedCornerShape(NhakoRadius.lg)).background(Nhako.colors.surface).padding(padding)) {
        content()
    }
}

/** A text field with the sunken look. [secret] hides it (tokens). */
@Composable
fun NTextField(
    value: String,
    onChange: (String) -> Unit,
    modifier: Modifier = Modifier,
    placeholder: String = "",
    secret: Boolean = false,
    align: TextAlign = TextAlign.Start,
    style: TextStyle = NhakoType.body.style(Nhako.colors.textPrimary),
    capitalize: Boolean = false,
    imeAction: ImeAction = ImeAction.Done,
    onDone: () -> Unit = {},
    error: Boolean = false,
    trailing: @Composable (() -> Unit)? = null,
    /** What a screen reader calls the field ("Device name"). Without it, the placeholder. */
    label: String = placeholder,
) {
    val c = Nhako.colors
    val border by animateColorAsState(if (error) c.danger else Color.Transparent, tween(NhakoMotion.Duration.fast), label = "border")
    // Focus draws a thin "charge" line along the bottom, left to right. Reduced motion: it fades.
    val interaction = remember { MutableInteractionSource() }
    val focused by interaction.collectIsFocusedAsState()
    val reduced = Nhako.reducedMotion
    val charge by animateFloatAsState(if (focused) 1f else 0f, if (reduced) tween(NhakoMotion.Duration.fast) else NhakoMotion.smooth.spec(), label = "charge")
    BasicTextField(
        interactionSource = interaction,
        value = value,
        onValueChange = onChange,
        singleLine = true,
        textStyle = style.copy(textAlign = align),
        cursorBrush = SolidColor(c.accent),
        visualTransformation = if (secret) PasswordVisualTransformation() else VisualTransformation.None,
        keyboardOptions = KeyboardOptions(capitalization = if (capitalize) KeyboardCapitalization.Words else KeyboardCapitalization.None, autoCorrectEnabled = false, imeAction = imeAction),
        keyboardActions = KeyboardActions(onDone = { onDone() }, onGo = { onDone() }),
        modifier = if (label.isEmpty()) modifier else modifier.semantics { contentDescription = label },
        decorationBox = { inner ->
            Box {
                Row(
                    Modifier.fillMaxWidth().heightIn(min = 48.dp).clip(RoundedCornerShape(NhakoRadius.sm)).background(c.surfaceSunken)
                        .border(1.5.dp, border, RoundedCornerShape(NhakoRadius.sm)).padding(horizontal = NhakoSpace.s4),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Box(Modifier.weight(1f), contentAlignment = when (align) { TextAlign.Center -> Alignment.Center; TextAlign.End, TextAlign.Right -> Alignment.CenterEnd; else -> Alignment.CenterStart }) {
                        if (value.isEmpty()) BasicText(placeholder, style = style.copy(color = c.textTertiary, textAlign = align), maxLines = 1, overflow = TextOverflow.Ellipsis)
                        inner()
                    }
                    trailing?.invoke()
                }
                if (charge > 0.001f) Box(
                    Modifier.matchParentSize().drawWithContent {
                        val inset = NhakoSpace.s4.toPx()
                        val w = (size.width - 2 * inset) * (if (reduced) 1f else charge)
                        drawRoundRect(c.accent, Offset(inset, size.height - 2.dp.toPx()), Size(w, 2.dp.toPx()), CornerRadius(1.dp.toPx()), alpha = if (reduced) charge else 1f)
                    },
                )
            }
        },
    )
}

/** A text box for a few lines (Send text), with the same sunken look and charge line as [NTextField]. */
@Composable
fun NTextArea(value: String, onChange: (String) -> Unit, modifier: Modifier = Modifier, placeholder: String = "", error: Boolean = false, minLines: Int = 5) {
    val c = Nhako.colors
    val style = NhakoType.body.style(c.textPrimary)
    val interaction = remember { MutableInteractionSource() }
    val focused by interaction.collectIsFocusedAsState()
    val reduced = Nhako.reducedMotion
    val charge by animateFloatAsState(if (focused) 1f else 0f, if (reduced) tween(NhakoMotion.Duration.fast) else NhakoMotion.smooth.spec(), label = "charge")
    val line = if (error) c.danger else c.accent
    BasicTextField(
        interactionSource = interaction,
        value = value,
        onValueChange = onChange,
        textStyle = style,
        minLines = minLines,
        maxLines = 12,
        cursorBrush = SolidColor(c.accent),
        modifier = modifier,
        decorationBox = { inner ->
            Box(
                Modifier.fillMaxWidth().clip(RoundedCornerShape(NhakoRadius.md)).background(c.surfaceSunken)
                    .padding(horizontal = NhakoSpace.s4, vertical = NhakoSpace.s3)
                    .drawWithContent {
                        drawContent()
                        if (charge <= 0.001f) return@drawWithContent
                        val w = size.width * (if (reduced) 1f else charge)
                        drawRoundRect(line, Offset(0f, size.height + NhakoSpace.s3.toPx() - 2.dp.toPx()), Size(w, 2.dp.toPx()), CornerRadius(1.dp.toPx()), alpha = if (reduced) charge else 1f)
                    },
            ) {
                if (value.isEmpty()) BasicText(placeholder, style = style.copy(color = c.textTertiary))
                inner()
            }
        },
    )
}

/** The section title above a settings card. */
@Composable
fun SectionTitle(text: String, modifier: Modifier = Modifier) {
    BasicText(text, modifier.padding(start = NhakoSpace.s2, bottom = NhakoSpace.s3), style = NhakoType.footnote.style(Nhako.colors.textSecondary).copy(fontWeight = androidx.compose.ui.text.font.FontWeight(600)))
}

@Composable
fun Separator(modifier: Modifier = Modifier) {
    Box(modifier.fillMaxWidth().height(1.dp).background(Nhako.colors.separator))
}

/** Small rounded label, like the route badge ("Nearby"). */
@Composable
fun Badge(text: String, icon: String?, bg: Color, fg: Color, modifier: Modifier = Modifier) {
    Row(modifier.clip(RoundedCornerShape(NhakoRadius.xs)).background(bg).padding(horizontal = 6.dp, vertical = 2.dp), verticalAlignment = Alignment.CenterVertically) {
        if (icon != null) {
            PhIcon(icon, size = 12.dp, tint = fg, weight = IconWeight.Bold)
            Spacer(Modifier.width(3.dp))
        }
        BasicText(text, style = NhakoType.caption.style(fg))
    }
}

/** Long file lists show this many at a time, so a transfer of thousands of files opens at once
 * (desktop: `FILES_PAGE` in Sheets.svelte). */
const val FILES_PAGE = 100

/** "Show 100 more" under a file list that has more than it shows. */
@Composable
fun MoreFiles(total: Int, shown: Int, onMore: () -> Unit) {
    if (total <= shown) return
    Box(Modifier.fillMaxWidth(), contentAlignment = Alignment.Center) {
        NButton(t("transfer.files.more", "count" to minOf(FILES_PAGE, total - shown)), onMore, variant = ButtonVariant.Plain, size = ButtonSize.Sm, icon = "plus")
    }
}
