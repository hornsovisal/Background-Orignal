plugins {
    alias(libs.plugins.agp.app)
    alias(libs.plugins.kotlin)
}

android {
    namespace = "ca.concordia.madiba.cryptologger"
    compileSdk = 35
//    buildToolsVersion = "34.0.0"

    defaultConfig {
        minSdk = 33
        targetSdk = 35
        versionCode = 1
        versionName = "1.0"
    }

    buildTypes {
        release {
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles("proguard-rules.pro")
            signingConfig = signingConfigs["debug"]
        }
    }

    buildFeatures {
        viewBinding = true
    }

//    kotlin {
//        jvmToolchain(17)
//    }
    kotlinOptions {
        jvmTarget = "17"
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    packaging {
        resources {
            merges += "META-INF/xposed/*"
            excludes += "**"
        }
    }

    lint {
        abortOnError = true
        checkReleaseBuilds = false
        baseline = file("lint-baseline.xml")
    }

}

dependencies {
//    implementation(libs.work.runtime.ktx)
    implementation(libs.reflections)

//    implementation(libs.firebase.crashlytics.buildtools)
    compileOnly(libs.libxposed.api)
    implementation(libs.libxposed.service)
    implementation(libs.okhttp)
    implementation(libs.commons.compress)
    implementation(libs.okhttp.logging.interceptor)



}
