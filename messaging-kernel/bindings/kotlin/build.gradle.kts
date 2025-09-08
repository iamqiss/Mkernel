plugins {
    kotlin("multiplatform") version "1.9.20"
    kotlin("plugin.serialization") version "1.9.20"
    id("com.android.library") version "8.1.2"
    id("org.jetbrains.compose") version "1.5.4"
}

kotlin {
    androidTarget {
        compilations.all {
            kotlinOptions {
                jvmTarget = "1.8"
            }
        }
    }
    
    jvm("desktop")
    
    sourceSets {
        val commonMain by getting {
            dependencies {
                implementation("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.7.3")
                implementation("org.jetbrains.kotlinx:kotlinx-serialization-json:1.6.0")
                implementation("org.jetbrains.kotlinx:kotlinx-datetime:0.4.1")
                implementation("io.ktor:ktor-client-core:2.3.5")
                implementation("io.ktor:ktor-client-cio:2.3.5")
                implementation("io.ktor:ktor-client-websockets:2.3.5")
                implementation("io.ktor:ktor-client-logging:2.3.5")
                implementation("io.ktor:ktor-client-content-negotiation:2.3.5")
                implementation("io.ktor:ktor-serialization-kotlinx-json:2.3.5")
                implementation("io.ktor:ktor-client-auth:2.3.5")
                implementation("io.ktor:ktor-client-encoding:2.3.5")
                implementation("io.ktor:ktor-client-encoding-jvm:2.3.5")
            }
        }
        
        val androidMain by getting {
            dependencies {
                implementation("androidx.lifecycle:lifecycle-runtime-ktx:2.7.0")
                implementation("androidx.lifecycle:lifecycle-viewmodel-ktx:2.7.0")
            }
        }
        
        val desktopMain by getting {
            dependencies {
                implementation("io.ktor:ktor-client-cio:2.3.5")
            }
        }
    }
}

android {
    namespace = "com.neo.protocol"
    compileSdk = 34
    
    defaultConfig {
        minSdk = 21
        targetSdk = 34
    }
    
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_1_8
        targetCompatibility = JavaVersion.VERSION_1_8
    }
    
    kotlinOptions {
        jvmTarget = "1.8"
    }
}