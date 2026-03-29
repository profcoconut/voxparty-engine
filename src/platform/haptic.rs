/// Haptic (vibration) feedback manager.
///
/// On desktop: no-op stub.
/// On mobile (iOS/Android): uses SDL_haptic via sdl2.

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
        fn default() -> Self { Self::new(None) }
    }
}

#[cfg(any(target_os = "ios", target_os = "android"))]
mod platform_haptic {
    /// Mobile haptic manager — drives SDL_haptic.
    pub struct HapticManager {
        haptic: Option<sdl2::HapticSubsystem>,
    }

    impl HapticManager {
        pub fn new(haptic: Option<sdl2::HapticSubsystem>) -> Self {
            Self { haptic }
        }

        /// Trigger a vibration for `millis` duration with `intensity` in [0.0, 1.0].
        pub fn vibrate(&mut self, millis: u32, _intensity: f32) {
            if let Some(ref mut h) = self.haptic {
                let _ = h.rumble_play(0, millis as u32);
            }
        }
    }
}

pub use platform_haptic::HapticManager;
