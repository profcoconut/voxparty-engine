pub mod sdl2;
pub mod touch;
pub mod audio;
pub mod haptic;

pub use sdl2::{Platform, RawSprite};
pub use touch::{TouchHandler, GameInput, VirtualGamepad};
pub use audio::AudioManager;
pub use haptic::HapticManager;
