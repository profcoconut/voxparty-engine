/// Haptic (vibration) feedback manager.
///
/// On desktop: no-op stub.
/// On iOS: Core Haptics via core-haptics crate.
/// On Android: no-op stub (Phase 7 will add ndk Vibrator).

#[cfg(not(any(target_os = "ios", target_os = "android")))]
mod platform_haptic {
    /// Desktop haptic manager — all operations are no-ops.
    pub struct HapticManager {
        _dummy: (),
    }

    impl HapticManager {
        pub fn new(_haptic: Option<sdl2::HapticSubsystem>) -> Self {
            Self { _dummy: () }
        }

        /// No-op on desktop.
        pub fn vibrate(&mut self, _millis: u32, _intensity: f32) {}
    }

    impl Default for HapticManager {
        fn default() -> Self {
            Self::new(None)
        }
    }
}

#[cfg(target_os = "ios")]
mod platform_haptic {
    use objc2_core_haptics::{CHHapticEngine, CHHapticEvent, CHHapticEventParameters};

    /// iOS haptic manager — uses Core Haptics.
    pub struct HapticManager {
        engine: Option<CHHapticEngine>,
    }

    impl HapticManager {
        pub fn new(_haptic: Option<sdl2::HapticSubsystem>) -> Result<Self, String> {
            // Ignore SDL haptic — use native Core Haptics instead.
            // SDL_haptic is a no-op on iOS per SDL2 docs.
            match CHHapticEngine::new() {
                Ok(engine) => Ok(Self { engine: Some(engine) }),
                Err(e) => Err(format!("Core Haptics unavailable: {:?}", e)),
            }
        }

        /// Trigger a vibration for `millis` duration with `intensity` in [0.0, 1.0].
        pub fn vibrate(&mut self, millis: u32, intensity: f32) {
            if let Some(ref mut eng) = self.engine {
                let params = CHHapticEventParameters::new(intensity, 0.5);
                let evt = CHHapticEvent::new(
                    objc2_core_haptics::CHHapticEvent::FIELD_RUMBLE,
                    &params,
                    0.0,
                    millis as f32 / 1000.0,
                );
                let _ = eng.start();
                let _ = eng.send_events(&[evt]);
            }
        }
    }

    impl Default for HapticManager {
        fn default() -> Self {
            Self::new(None).expect("Core Haptics should be available on iOS")
        }
    }
}

#[cfg(target_os = "android")]
mod platform_haptic {
    /// Android haptic manager — no-op stub for Phase 5.
    /// Phase 7 will implement ndk Vibrator API.
    pub struct HapticManager {
        _dummy: (),
    }

    impl HapticManager {
        pub fn new(_haptic: Option<sdl2::HapticSubsystem>) -> Result<Self, String> {
            // Phase 5: no-op stub. Phase 7 will use ndk Vibrator.
            Ok(Self { _dummy: () })
        }

        /// No-op on Android for Phase 5.
        pub fn vibrate(&mut self, _millis: u32, _intensity: f32) {}
    }

    impl Default for HapticManager {
        fn default() -> Self {
            Self::new(None).expect("Android haptic stub should always succeed")
        }
    }
}

pub use platform_haptic::HapticManager;
