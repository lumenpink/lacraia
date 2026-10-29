import java.util.Properties
plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

val canonicalVersion: String = run {
    val versionFile = rootProject.file("../../VERSION")
    if (versionFile.exists()) {
        val line = versionFile.readText().lines().firstOrNull { it.isNotBlank() }?.trim()
        if (!line.isNullOrEmpty()) {
            return@run line
        }
    }
    (project.findProperty("versionName") as? String) ?: System.getenv("VERSION_NAME") ?: "0.2.0"
}

val canonicalVersionCode: Int = run {
    val prop = project.findProperty("versionCode")?.toString()?.toIntOrNull()
    if (prop != null) return@run prop
    val env = System.getenv("VERSION_CODE")?.toIntOrNull()
    if (env != null) return@run env
    val devMatch = Regex("""-dev\.(\d+)""").find(canonicalVersion)
    if (devMatch != null) {
        return@run devMatch.groupValues[1].toInt()
    }
    val semverMatch = Regex("""^(\d+)\.(\d+)\.(\d+)""").find(canonicalVersion)
    if (semverMatch != null) {
        val (major, minor, patch) = semverMatch.destructured
        return@run major.toInt() * 10000 + minor.toInt() * 100 + patch.toInt()
    }
    1
}

android {
    namespace = "in.twolp.lacraia.simbiotyka"
    compileSdk = 34

    defaultConfig {
        applicationId = "in.twolp.lacraia.simbiotyka"
        minSdk = 26
        targetSdk = 34
        versionCode = canonicalVersionCode
        versionName = canonicalVersion

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    signingConfigs {
        create("release") {
            val userHome = System.getProperty("user.home")
            val keystoreProps = file("$userHome/.android/in_twolp_release.properties")
            val releaseKeystore = file("$userHome/.android/in_twolp_release.keystore")
            val debugKeystore = file("$userHome/.android/debug.keystore")

            if (keystoreProps.exists() && releaseKeystore.exists()) {
                val props = Properties().apply {
                    keystoreProps.inputStream().use { load(it) }
                }
                storeFile = releaseKeystore
                storePassword = props.getProperty("storePassword")
                keyAlias = props.getProperty("keyAlias", "in_twolp")
                keyPassword = props.getProperty("keyPassword")
            } else if (debugKeystore.exists()) {
                storeFile = debugKeystore
                storePassword = "android"
                keyAlias = "androiddebugkey"
                keyPassword = "android"
            }
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            signingConfig = signingConfigs.getByName("release")
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions {
        jvmTarget = "17"
    }
    buildFeatures {
        viewBinding = true
    }
}

dependencies {
    implementation("androidx.core:core-ktx:1.13.1")
    implementation("androidx.appcompat:appcompat:1.7.0")
    implementation("com.google.android.material:material:1.12.0")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.8.1")
    implementation("com.squareup.okhttp3:okhttp:4.12.0")
    implementation("org.json:json:20240303")
    testImplementation("junit:junit:4.13.2")
    testImplementation("com.squareup.okhttp3:mockwebserver:4.12.0")
}

