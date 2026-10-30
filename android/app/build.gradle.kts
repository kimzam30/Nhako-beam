import java.util.Properties
import javax.inject.Inject
import org.gradle.process.ExecOperations

plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.compose)
    alias(libs.plugins.kotlin.serialization)
    alias(libs.plugins.roborazzi)
}

/**
 * The F-Droid build: `-Pnhako.fdroid`. F-Droid only takes free software, so it leaves out Firebase
 * (no wake-up while closed, src/nopush/ instead of src/push/) and the check for new versions on
 * GitHub (F-Droid updates the app itself). Everything else, and every task name, is the same.
 */
val fdroid: Boolean = findProperty("nhako.fdroid") != null

// Push wake-up (wake/README.md) needs your Firebase project's google-services.json here. Without
// it the app builds and works the same, it just can't be woken while closed.
if (!fdroid && file("google-services.json").exists()) apply(plugin = "com.google.gms.google-services")

/** The repository root: the Rust workspace (core/, ffi/) and design/tokens live there. */
val repoRoot: File = rootProject.projectDir.parentFile

/** Phone architectures to build the engine for. Faster local builds: `-Pnhako.abis=arm64-v8a`. */
val engineAbis: List<String> = (findProperty("nhako.abis") as String? ?: "arm64-v8a,armeabi-v7a,x86_64").split(",").map { it.trim() }

/**
 * The release key (docs/releasing.md): from the NHAKO_KEYSTORE_* variables (the Release workflow)
 * or android/keystore.properties (your computer). Android only installs an update signed with the
 * same key as the app already on the phone, so every published APK must use this one key.
 */
val keystoreProperties = Properties().apply {
    rootProject.file("keystore.properties").takeIf { it.exists() }?.inputStream()?.use { load(it) }
}
fun releaseKey(env: String, property: String): String? =
    System.getenv(env)?.takeIf { it.isNotBlank() } ?: keystoreProperties.getProperty(property)
val releaseStoreFile: String? = releaseKey("NHAKO_KEYSTORE_FILE", "storeFile")

android {
    namespace = "io.github.kimzam30.nhakobeam"
    compileSdk = 37
    ndkVersion = "29.0.14206865"

    defaultConfig {
        applicationId = "io.github.kimzam30.nhakobeam"
        minSdk = 26
        targetSdk = 37
        versionCode = 1
        versionName = "0.1.0"
        buildConfigField("boolean", "FROM_STORE", fdroid.toString())
    }

    // One APK per phone type (Phase 8): each carries only its own copy of the engine, about half
    // the size of one APK for all of them. Almost every phone today is arm64-v8a.
    splits {
        abi {
            // One APK per phone type. Not for an app bundle (.aab): Play makes the split itself, and
            // with splits on, shrinking leaves one resource file per type and bundleRelease fails.
            isEnable = gradle.startParameter.taskNames.none { it.contains("bundle", ignoreCase = true) }
            reset()
            include(*engineAbis.toTypedArray())
            isUniversalApk = false
        }
    }

    signingConfigs {
        if (releaseStoreFile != null) {
            create("release") {
                storeFile = rootProject.file(releaseStoreFile)
                storePassword = releaseKey("NHAKO_KEYSTORE_PASSWORD", "storePassword")
                keyAlias = releaseKey("NHAKO_KEY_ALIAS", "keyAlias")
                // A PKCS12 keystore (keytool's default) has one password for the store and the key.
                keyPassword = releaseKey("NHAKO_KEY_PASSWORD", "keyPassword") ?: storePassword
            }
        }
    }

    buildTypes {
        debug { applicationIdSuffix = ".debug" }
        release {
            // R8 drops the code and resources the app never uses: the Kotlin side shrinks from
            // about 17 MB to a few. Rules for the engine bindings are in proguard-rules.pro.
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
            // Without the release key, this computer's debug key: fine for trying things, never for
            // publishing (the Release workflow refuses to build without the real key).
            // F-Droid signs what it builds itself, so its build comes out unsigned.
            signingConfig = if (fdroid) null else signingConfigs.findByName("release") ?: signingConfigs.getByName("debug")
        }
    }

    lint {
        // A false alarm: CameraX asks for an old Fragment, but 1.5.4 is what ships (well past the
        // 1.3.0 it wants), and MainActivity is a ComponentActivity, not a FragmentActivity.
        disable += "InvalidFragmentVersionForActivityResult"
    }

    buildFeatures {
        compose = true
        buildConfig = true
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    testOptions {
        unitTests {
            isIncludeAndroidResources = true
            all {
                it.systemProperty("robolectric.pixelCopyRenderMode", "hardware")
                // Screenshots are for looking at, not pixel comparison, so always write them.
                it.systemProperty("roborazzi.test.record", "true")
                // RealEngineTest loads the host copy of the engine that the bindings are made from.
                it.systemProperty("jna.library.path", repoRoot.resolve("target/debug").path)
                // Robolectric's Android 16 runtime reaches into FileDescriptor internals.
                it.jvmArgs("--add-exports=java.base/jdk.internal.access=ALL-UNNAMED", "--add-opens=java.base/java.io=ALL-UNNAMED")
                // Robolectric downloads the Android runtime itself; a machine that is rate limited
                // by Maven Central can point it at a mirror in ~/.gradle/gradle.properties.
                providers.gradleProperty("nhako.robolectricRepo").orNull?.let { url ->
                    it.systemProperty("robolectric.dependency.repo.url", url)
                }
            }
        }
    }
}

kotlin { compilerOptions { jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17) } }

/** Builds libnhako_ffi.so for each architecture with cargo-ndk (`cargo install cargo-ndk`). */
abstract class CargoNdkTask : DefaultTask() {
    @get:Inject abstract val exec: ExecOperations
    @get:Input abstract val abis: ListProperty<String>
    @get:Input abstract val ndkDir: Property<String>
    /** The wake service built into the engine (wake/README.md); empty for none. */
    @get:Input abstract val wakeUrl: Property<String>
    @get:Internal abstract val workspace: DirectoryProperty
    @get:InputFiles @get:PathSensitive(PathSensitivity.RELATIVE) abstract val sources: ConfigurableFileCollection
    @get:OutputDirectory abstract val outputDir: DirectoryProperty

    @TaskAction
    fun build() {
        val targets = abis.get().flatMap { listOf("-t", it) }
        exec.exec {
            workingDir = workspace.get().asFile
            environment("ANDROID_NDK_HOME", ndkDir.get())
            environment("NHAKO_WAKE_URL", wakeUrl.get())
            commandLine(listOf("cargo", "ndk") + targets + listOf("-P", "26", "-o", outputDir.get().asFile.path, "build", "--release", "-p", "nhako-ffi"))
        }
        // Some dependencies also build as shared libraries; the app only needs the engine.
        outputDir.get().asFile.walk().filter { it.isFile && it.name != "libnhako_ffi.so" }.forEach { it.delete() }
    }
}

/**
 * Generates the Kotlin bindings (io.github.kimzam30.nhakobeam.core). They're read from the engine
 * built for this machine: the Android libraries are stripped, which drops the metadata UniFFI reads,
 * and the bindings are the same for every platform. That build is also what the JVM tests load.
 */
abstract class UniffiBindingsTask : DefaultTask() {
    @get:Inject abstract val exec: ExecOperations
    @get:Internal abstract val workspace: DirectoryProperty
    @get:InputFiles @get:PathSensitive(PathSensitivity.RELATIVE) abstract val sources: ConfigurableFileCollection
    @get:OutputDirectory abstract val outputDir: DirectoryProperty

    @TaskAction
    fun generate() {
        val out = outputDir.get().asFile
        out.deleteRecursively()
        exec.exec {
            workingDir = workspace.get().asFile
            commandLine("cargo", "build", "--quiet", "-p", "nhako-ffi", "--lib")
        }
        val library = workspace.get().asFile.resolve("target/debug/libnhako_ffi.so")
        exec.exec {
            workingDir = workspace.get().asFile
            commandLine(
                "cargo", "run", "--quiet", "-p", "nhako-ffi", "--bin", "uniffi-bindgen", "--",
                "generate", "--library", library.path, "--language", "kotlin", "--out-dir", out.path, "--no-format",
            )
        }
    }
}

val rustSources = fileTree(repoRoot) {
    include("core/src/**", "core/Cargo.toml", "ffi/src/**", "ffi/Cargo.toml", "ffi/uniffi.toml", "Cargo.toml", "Cargo.lock")
}

val cargoNdk = tasks.register<CargoNdkTask>("cargoNdk") {
    abis.set(engineAbis)
    ndkDir.set(androidComponents.sdkComponents.sdkDirectory.map { it.dir("ndk/${android.ndkVersion}").asFile.path })
    wakeUrl.set(providers.environmentVariable("NHAKO_WAKE_URL").orElse(""))
    workspace.set(repoRoot)
    sources.from(rustSources)
    outputDir.set(layout.buildDirectory.dir("rust/jniLibs"))
}

val uniffiBindings = tasks.register<UniffiBindingsTask>("uniffiBindings") {
    workspace.set(repoRoot)
    sources.from(rustSources)
    outputDir.set(layout.buildDirectory.dir("generated/uniffi/kotlin"))
}

// The packaging steps don't notice when the phone types change (say after a `-Pnhako.abis=arm64-v8a`
// build) and would keep only the old APKs, so the list is one of their inputs.
tasks.matching { it.name == "packageDebug" || it.name == "packageRelease" }.configureEach {
    inputs.property("nhako.abis", engineAbis.joinToString(","))
}

androidComponents {
    onVariants { variant ->
        variant.sources.jniLibs?.addGeneratedSourceDirectory(cargoNdk, CargoNdkTask::outputDir)
        variant.sources.kotlin?.addGeneratedSourceDirectory(uniffiBindings, UniffiBindingsTask::outputDir)
        // Colors, type, spacing, springs and haptics, shared with the desktop app.
        variant.sources.kotlin?.addStaticSourceDirectory(repoRoot.resolve("design/tokens/build").path)
        // The sounds (design/sounds/build.mjs): the WAVs as res/raw, and their names in Kotlin.
        variant.sources.res?.addStaticSourceDirectory(repoRoot.resolve("design/sounds/build/res").path)
        variant.sources.kotlin?.addStaticSourceDirectory(repoRoot.resolve("design/sounds/build/kotlin").path)
        // Wake-up by push, or (F-Droid) without it.
        variant.sources.kotlin?.addStaticSourceDirectory(file(if (fdroid) "src/nopush/kotlin" else "src/push/kotlin").path)
        if (!fdroid) variant.sources.manifests.addStaticManifestFile(file("src/push/AndroidManifest.xml").path)
    }
}

dependencies {
    implementation(platform(libs.compose.bom))
    implementation(libs.compose.ui)
    implementation(libs.compose.foundation)
    implementation(libs.compose.animation)
    implementation(libs.androidx.core.ktx)
    implementation(libs.androidx.core.splashscreen)
    if (!fdroid) {
        implementation(platform(libs.firebase.bom))
        implementation(libs.firebase.messaging)
    }
    implementation(libs.androidx.activity.compose)
    implementation(libs.androidx.lifecycle.runtime.compose)
    implementation(libs.camerax.camera2)
    implementation(libs.camerax.lifecycle)
    implementation(libs.camerax.view)
    implementation(libs.zxing.core)
    implementation(libs.kotlinx.serialization.json)
    implementation(libs.kotlinx.coroutines.android)
    implementation("${libs.jna.get()}@aar")
    debugImplementation(libs.compose.ui.test.manifest)

    testImplementation(libs.junit)
    testImplementation(libs.robolectric)
    testImplementation(platform(libs.compose.bom))
    testImplementation(libs.compose.ui.test.junit4)
    testImplementation(libs.roborazzi)
    testImplementation(libs.roborazzi.compose)
    testImplementation(libs.roborazzi.junit.rule)
    // The JVM tests load the engine built for this machine through plain JNA.
    testImplementation(libs.jna)
}
