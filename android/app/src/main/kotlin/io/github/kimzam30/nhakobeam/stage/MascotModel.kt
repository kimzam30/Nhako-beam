package io.github.kimzam30.nhakobeam.stage

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.boolean
import kotlinx.serialization.json.float
import kotlinx.serialization.json.int
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import java.nio.ByteBuffer
import java.nio.ByteOrder

/**
 * The voxel mascot and its stage, read from the app's assets (design/mascot/build/, made by
 * design/mascot/build.mjs): `mascot.glb` for the model, `mascot.json` for the states, the devices
 * and the camera. The desktop app reads the same two files.
 */
class MascotModel(val parts: List<Part>, val rootScale: Float, val rootLift: Float, val data: MascotData) {
    /** One moving part: triangles with a normal and a color for each corner, around [origin]. */
    class Part(
        val name: String,
        /** Where the part's middle is, in the model's units (voxels). */
        val origin: FloatArray,
        val hidden: Boolean,
        val positions: FloatArray,
        val normals: FloatArray,
        /** Linear RGB, 0 to 1, three per corner. */
        val colors: FloatArray,
        val indices: ShortArray,
    )

    fun part(name: String) = parts.find { it.name == name }

    companion object {
        fun load(glb: ByteArray, json: String): MascotModel = readGlb(glb, MascotData.parse(json))
    }
}

/** Reads the glTF binary our build writes: one buffer, a node per part under one root. */
private fun readGlb(bytes: ByteArray, data: MascotData): MascotModel {
    val b = ByteBuffer.wrap(bytes).order(ByteOrder.LITTLE_ENDIAN)
    require(b.getInt(0) == 0x46546c67) { "Not a glTF binary" }
    val jsonLen = b.getInt(12)
    val gltf = Json.parseToJsonElement(String(bytes, 20, jsonLen, Charsets.UTF_8)).jsonObject
    val binStart = 20 + jsonLen + 8
    val views = gltf["bufferViews"]!!.jsonArray
    val accessors = gltf["accessors"]!!.jsonArray

    fun view(accessor: Int): Pair<Int, JsonObject> {
        val a = accessors[accessor].jsonObject
        val v = views[a["bufferView"]!!.jsonPrimitive.int].jsonObject
        return binStart + (v["byteOffset"]?.jsonPrimitive?.int ?: 0) to a
    }
    fun floats(accessor: Int, per: Int): FloatArray {
        val (at, a) = view(accessor)
        val n = a["count"]!!.jsonPrimitive.int * per
        return FloatArray(n) { b.getFloat(at + it * 4) }
    }
    /** Byte colors, four per corner, as linear floats, three per corner. */
    fun colors(accessor: Int): FloatArray {
        val (at, a) = view(accessor)
        val n = a["count"]!!.jsonPrimitive.int
        return FloatArray(n * 3) { (bytes[at + (it / 3) * 4 + it % 3].toInt() and 0xff) / 255f }
    }
    fun shorts(accessor: Int): ShortArray {
        val (at, a) = view(accessor)
        return ShortArray(a["count"]!!.jsonPrimitive.int) { b.getShort(at + it * 2) }
    }

    val nodes = gltf["nodes"]!!.jsonArray
    val meshes = gltf["meshes"]!!.jsonArray
    val parts = mutableListOf<MascotModel.Part>()
    var rootScale = 1f
    var rootLift = 0f
    for (n in nodes) {
        val node = n.jsonObject
        val meshIndex = node["mesh"]?.jsonPrimitive?.int
        if (meshIndex == null) {
            rootScale = node["scale"]?.jsonArray?.get(0)?.jsonPrimitive?.float ?: 1f
            rootLift = node["translation"]?.jsonArray?.get(1)?.jsonPrimitive?.float ?: 0f
            continue
        }
        val prim = meshes[meshIndex].jsonObject["primitives"]!!.jsonArray[0].jsonObject
        val attrs = prim["attributes"]!!.jsonObject
        parts += MascotModel.Part(
            name = node["name"]!!.jsonPrimitive.content,
            origin = node["translation"]!!.jsonArray.map { it.jsonPrimitive.float }.toFloatArray(),
            hidden = node["extras"]?.jsonObject?.get("hidden")?.jsonPrimitive?.boolean ?: false,
            positions = floats(attrs["POSITION"]!!.jsonPrimitive.int, 3),
            normals = floats(attrs["NORMAL"]!!.jsonPrimitive.int, 3),
            colors = colors(attrs["COLOR_0"]!!.jsonPrimitive.int),
            indices = shorts(prim["indices"]!!.jsonPrimitive.int),
        )
    }
    return MascotModel(parts, rootScale, rootLift, data)
}

/** mascot.json: the states, which parts make each face, your devices as boxes, the camera. */
class MascotData(
    val states: Map<String, StateSpec>,
    val faces: Map<String, List<String>>,
    val devices: Map<String, List<DeviceBox>>,
    val deviceColors: Map<String, Int>,
    val stage: StageSpec,
) {
    class StateSpec(val id: String, val face: String, val motion: Map<String, Float>, val blinkRange: Pair<Float, Float>?, val pose: Map<String, Float>)
    class DeviceBox(val size: FloatArray, val at: FloatArray, val tiltDeg: Float, val role: String)
    class StageSpec(
        val fovDeg: Float, val eye: FloatArray, val target: FloatArray, val heightRatio: Float,
        val floorRadius: Float, val slotReach: Float, val mascotScale: Float, val deviceScale: Float,
    )

    companion object {
        fun parse(json: String): MascotData {
            val root = Json.parseToJsonElement(json).jsonObject
            fun nums(o: JsonObject?): Map<String, Float> = o.orEmpty().mapNotNull { (k, v) ->
                val p = v as? kotlinx.serialization.json.JsonPrimitive ?: return@mapNotNull null
                when {
                    p.isString -> null
                    p.content == "true" -> k to 1f
                    p.content == "false" -> k to 0f
                    else -> k to p.float
                }
            }.toMap()
            val states = root["states"]!!.jsonArray.associate { e ->
                val o = e.jsonObject
                val motion = o["motion"]?.jsonObject
                val blink = (motion?.get("blinkEveryMs") as? JsonArray)?.let { it[0].jsonPrimitive.float to it[1].jsonPrimitive.float }
                val id = o["id"]!!.jsonPrimitive.content
                id to StateSpec(id, o["face"]!!.jsonPrimitive.content, nums(motion), blink, nums(o["pose"]?.jsonObject))
            }
            val faces = root["faces"]!!.jsonObject.mapValues { (_, v) -> v.jsonArray.map { it.jsonPrimitive.content } }
            val devices = root["devices"]!!.jsonObject.mapValues { (_, v) ->
                v.jsonArray.map { e ->
                    val o = e.jsonObject
                    DeviceBox(
                        o["size"]!!.jsonArray.map { it.jsonPrimitive.float }.toFloatArray(),
                        o["at"]!!.jsonArray.map { it.jsonPrimitive.float }.toFloatArray(),
                        o["tilt"]?.jsonPrimitive?.float ?: 0f,
                        o["role"]!!.jsonPrimitive.content,
                    )
                }
            }
            val colors = root["deviceColors"]!!.jsonObject.mapValues { (_, v) -> 0xff000000.toInt() or v.jsonPrimitive.content.removePrefix("#").toInt(16) }
            val s = root["stage"]!!.jsonObject
            fun arr(k: String) = s[k]!!.jsonArray.map { it.jsonPrimitive.float }.toFloatArray()
            fun f(k: String) = s[k]!!.jsonPrimitive.float
            val stage = StageSpec(f("fovDeg"), arr("eye"), arr("target"), f("heightRatio"), f("floorRadius"), f("slotReach"), f("mascotScale"), f("deviceScale"))
            return MascotData(states, faces, devices, colors, stage)
        }
    }
}
