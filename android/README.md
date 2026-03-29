# VoxParty Android

## Build Rust Library

```bash
# Install Android NDK — required for cross-compilation
# Install via Android Studio SDK Manager or:
# brew install android-ndk

# Install Android target (once)
rustup target add aarch64-linux-android

# Build for Android arm64-v8a
cargo build --target aarch64-linux-android --release
```

The static library is at `target/aarch64-linux-android/release/libvoxparty.a`.

## Gradle Project Setup

1. Create `android/app/src/main/AndroidManifest.xml`:
```xml
<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="com.voxparty.game">
    <uses-permission android:name="android.permission.INTERNET" />
    <application android:label="VoxParty" android:hardwareAccelerated="true">
        <activity android:name=".MainActivity"
            android:configChanges="orientation|keyboardHidden|screenSize"
            android:launchMode="singleTask">
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
        </activity>
    </application>
</manifest>
```

2. Create `android/app/src/main/java/com/voxparty/game/MainActivity.kt`:
```kotlin
package com.voxparty.game

import android.app.Activity
import android.os.Bundle

class MainActivity : Activity() {
    init {
        System.loadLibrary("voxparty")
    }

    external fun nativeRun()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        nativeRun()
    }
}
```

3. Link the Rust static library in `android/app/build.gradle`:
```groovy
android {
    defaultConfig {
        ndk {
            abiFilters 'arm64-v8a'
        }
    }
}

dependencies {
    implementation project(':voxparty')
}
```

4. Add to `settings.gradle`:
```groovy
include ':voxparty'
project(':voxparty').name = 'voxparty'
project(':voxparty').buildDir = file('../target/aarch64-linux-android/release')
```

## Dependencies

SDL2 for Android is included in the NDK via ` SDL2/libs/` or can be built from source.

## Native Activity Alternative

For a simpler setup without JNI, use `android_native_app_glue` in `main.rs`:

```rust
#![cfg(target_os = "android")]
#![no_main]

use android_native_app_glue::android_main;

android_main!(|app| {
    // same platform init as desktop
    let sdl = sdl2::init().unwrap();
    voxparty::run_android(sdl, app);
});
```
