// Buttons, switches, segmented controls, banners and fields, with the press springs from the
// motion tokens. The Compose twins of desktop/src/components/Button.svelte, Toggle.svelte, etc.
package io.github.kimzam30.nhakobeam.ui.components

import androidx.compose.animation.animateColorAsState
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
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.scale
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
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
import io.github.kimzam30.nhakobeam.design.NhakoHaptic
import io.github.kimzam30.nhakobeam.design.NhakoMotion
import io.github.kimzam30.nhakobeam.design.NhakoRadius
import io.github.kimzam30.nhakobeam.design.NhakoSound
import io.github.kimzam30.nhakobeam.design.NhakoSpace
import io.github.kimzam30.nhakobeam.design.NhakoType
import io.github.kimzam30.nhakobeam.ui.LocalSound
import io.github.kimzam30.nhakobeam.ui.theme.IconWeight
import io.github.kimzam30.nhakobeam.ui.theme.Nhako
import io.github.kimzam30.nhakobeam.ui.theme.PhIcon
import io.github.kimzam30.nhakobeam.ui.theme.style
import io.github.kimzam30.nhakobeam.ui.withTap

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
) {
    val c = Nhako.colors
    val interaction = remember { MutableInteractionSource() }
    val play = LocalSound.current
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
            .pressScale(interaction)
            .alpha(if (enabled) 1f else 0.45f)
            .clip(RoundedCornerShape(NhakoRadius.full))
            .background(bg)
            .clickable(interaction, indication = null, enabled = active, role = Role.Button) {
                play(sound)
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

@Composable
fun NIconButton(icon: String, label: String, onClick: () -> Unit, modifier: Modifier = Modifier, tint: Color = Nhako.colors.textSecondary, size: Dp = 22.dp, weight: IconWeight = IconWeight.Duotone, enabled: Boolean = true, sound: NhakoSound = NhakoSound.Tap) {
    val interaction = remember { MutableInteractionSource() }
    val play = LocalSound.current
    Box(
        modifier
            .size(48.dp)
            .pressScale(interaction, 0.9f)
            .clip(CircleShape)
            .clickable(interaction, indication = null, enabled = enabled, role = Role.Button) {
                play(sound)
                onClick()
            }
            .semantics { contentDescription = label },
        contentAlignment = Alignment.Center,
    ) {
        PhIcon(icon, size = size, tint = if (enabled) tint else tint.copy(alpha = 0.4f), weight = weight)
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
    val track by animateColorAsState(if (checked) c.accent else c.fillPressed, tween(NhakoMotion.Duration.fast), label = "track")
    Row(
        modifier
            .fillMaxWidth()
            .heightIn(min = 52.dp)
            .toggleable(checked, role = Role.Switch) {
                (if (it) NhakoHaptic.ToggleOn else NhakoHaptic.ToggleOff).perform(view)
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
            Box(Modifier.offset { IntOffset(x.roundToPx(), 2.dp.roundToPx()) }.size(24.dp).shadow(2.dp, CircleShape).clip(CircleShape).background(Color.White))
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
    BoxWithConstraints(modifier.fillMaxWidth().height(40.dp).clip(RoundedCornerShape(NhakoRadius.sm)).background(c.fill).padding(3.dp)) {
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
                    if (o.icon != null) {
                        PhIcon(o.icon, size = 16.dp, tint = if (on) c.textPrimary else c.textSecondary, weight = if (on) IconWeight.Fill else IconWeight.Duotone)
                        Spacer(Modifier.width(NhakoSpace.s2))
                    }
                    BasicText(o.label, style = NhakoType.callout.style(if (on) c.textPrimary else c.textSecondary).copy(fontWeight = androidx.compose.ui.text.font.FontWeight(if (on) 600 else 500)))
                }
            }
        }
    }
}

enum class BannerTone { Warning, Danger, Info }

@Composable
fun NBanner(text: String, modifier: Modifier = Modifier, icon: String = "warning-circle", tone: BannerTone = BannerTone.Warning, actionLabel: String? = null, onAction: (() -> Unit)? = null) {
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
) {
    val c = Nhako.colors
    val border by animateColorAsState(if (error) c.danger else Color.Transparent, tween(NhakoMotion.Duration.fast), label = "border")
    BasicTextField(
        value = value,
        onValueChange = onChange,
        singleLine = true,
        textStyle = style.copy(textAlign = align),
        cursorBrush = SolidColor(c.accent),
        visualTransformation = if (secret) PasswordVisualTransformation() else VisualTransformation.None,
        keyboardOptions = KeyboardOptions(capitalization = if (capitalize) KeyboardCapitalization.Words else KeyboardCapitalization.None, autoCorrectEnabled = false, imeAction = imeAction),
        keyboardActions = KeyboardActions(onDone = { onDone() }, onGo = { onDone() }),
        modifier = modifier,
        decorationBox = { inner ->
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
