plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

android {
    namespace = "in._2lp.lacraia.app"
    compileSdk = 34

    defaultConfig {
        applicationId = "in.twolp.lacraia.app"
        minSdk = 26
        targetSdk = 34
        versionCode = 1
        versionName = "0.2.0"

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    signingConfigs {
        create("release") {
            val userHome = System.getProperty("user.home")
            val keystoreProps = file("$userHome/.android/in_twolp_release.properties")
            val releaseKeystore = file("$userHome/.android/in_twolp_release.keystore")
            val debugKeystore = file("$userHome/.android/debug.keystore")

            if (keystoreProps.exists() && releaseKeystore.exists()) {
                val props = java.util.Properties().apply {
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
    implementation("androidx.webkit:webkit:1.11.0")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.8.1")
    implementation("com.squareup.okhttp3:okhttp:4.12.0")
    implementation("org.json:json:20240303")
    testImplementation("junit:junit:4.13.2")
}
