package io.github.kimzam30.nhakobeam.engine

import android.content.ContentResolver
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.net.Uri
import android.os.Build
import android.util.Base64
import android.util.Size
import io.github.kimzam30.nhakobeam.core.SendThumb
import java.io.ByteArrayOutputStream
import kotlin.math.max
import kotlin.math.roundToInt

/**
 * Small previews of photos and videos you're about to send: shown in Send to, and sent with the
 * request so the other device sees what's coming. The desktop makes the same (src-tauri thumbs.rs),
 * photos only. At most 160 px on the long side and 32 KiB, as the engine requires.
 */
object Thumbs {
    const val SIDE = 160
    const val MAX = 4
    private const val BYTES_MAX = 32 * 1024

    /** A preview of [uri], or null if Android can't make one (not a photo or video, or unreadable). */
    fun make(resolver: ContentResolver, uri: Uri, mime: String): SendThumb? {
        val video = mime.startsWith("video/")
        if (!video && !mime.startsWith("image/")) return null
        val bitmap = runCatching {
            if (Build.VERSION.SDK_INT >= 29) resolver.loadThumbnail(uri, Size(SIDE, SIDE), null)
            else if (!video) decodeSmall(resolver, uri) else null
        }.getOrNull() ?: return null
        return encode(fit(bitmap))
    }

    /** Before Android 10: decode a photo at a fraction of its size, never the whole thing. */
    private fun decodeSmall(resolver: ContentResolver, uri: Uri): Bitmap? {
        val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
        resolver.openInputStream(uri)?.use { BitmapFactory.decodeStream(it, null, bounds) }
        if (bounds.outWidth <= 0 || bounds.outHeight <= 0) return null
        var sample = 1
        while (max(bounds.outWidth, bounds.outHeight) / (sample * 2) >= SIDE) sample *= 2
        return resolver.openInputStream(uri)?.use { BitmapFactory.decodeStream(it, null, BitmapFactory.Options().apply { inSampleSize = sample }) }
    }

    private fun fit(b: Bitmap): Bitmap {
        val long = max(b.width, b.height)
        if (long <= SIDE) return b
        val k = SIDE.toFloat() / long
        return Bitmap.createScaledBitmap(b, (b.width * k).roundToInt().coerceAtLeast(1), (b.height * k).roundToInt().coerceAtLeast(1), true)
    }

    private fun encode(b: Bitmap): SendThumb? {
        for (quality in intArrayOf(72, 55, 40)) {
            val out = ByteArrayOutputStream()
            if (!b.compress(Bitmap.CompressFormat.JPEG, quality, out)) return null
            val jpeg = out.toByteArray()
            if (jpeg.size <= BYTES_MAX) return SendThumb(jpeg, b.width.toUShort(), b.height.toUShort())
        }
        return null
    }

    /** The preview as the app shows it, the same as the engine gives for an incoming request. */
    fun dataUrl(t: SendThumb): String = "data:image/jpeg;base64," + Base64.encodeToString(t.jpeg, Base64.NO_WRAP)
}
