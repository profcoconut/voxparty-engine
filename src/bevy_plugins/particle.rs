//! Particle system for Bevy ECS — visual juice for movement, checkpoints, and traps.
//!
//! Particles are short-lived (0.3–1s), spawned as lightweight entities with a Sprite
//! and Transform. They are updated each frame by `particle_tick_system` and their
//! transform is synced by `particle_sprite_system`.
//!
//! Types:
//! - **Dust**: grey puff at player's feet when they move
//! - **Sparkle**: golden/white particles burst upward from checkpoint
//! - **Flash**: red ring that expands outward on trap trigger

use bevy::prelude::*;

/// RGBA color for a particle, stored as a simple tuple for easy copying.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParticleColor(pub u8, pub u8, pub u8, pub u8);

impl ParticleColor {
    pub fn golden() -> Self {
        Self(255, 215, 80, 255)
    }

    pub fn white() -> Self {
        Self(255, 255, 255, 255)
    }

    pub fn grey() -> Self {
        Self(180, 180, 180, 200)
    }

    pub fn red() -> Self {
        Self(255, 60, 60, 255)
    }

    /// Interpolate between two colors.
    fn lerp(self, other: ParticleColor, t: f32) -> ParticleColor {
        let t = t.clamp(0.0, 1.0);
        ParticleColor(
            (self.0 as f32 + (other.0 as f32 - self.0 as f32) * t) as u8,
            (self.1 as f32 + (other.1 as f32 - self.1 as f32) * t) as u8,
            (self.2 as f32 + (other.2 as f32 - self.2 as f32) * t) as u8,
            (self.3 as f32 + (other.3 as f32 - self.3 as f32) * t) as u8,
        )
    }

    /// Returns a Bevy Color from this ParticleColor.
    fn to_bevy_color(&self) -> Color {
        Color::srgba_u8(self.0, self.1, self.2, self.3)
    }

    /// Returns fade-out alpha based on remaining lifetime ratio.
    fn fade_alpha(&self, lifetime_ratio: f32) -> u8 {
        ((self.3 as f32) * lifetime_ratio.clamp(0.0, 1.0)) as u8
    }
}

/// Particle type — determines visual behavior and color interpolation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticleType {
    /// Small grey puff at player's feet when they move.
    Dust,
    /// Golden/white sparkle burst — rises upward from checkpoint.
    Sparkle,
    /// Red ring that expands outward — triggered on trap.
    Flash,
    /// General sparkle used for trap sparks.
    Spark,
}

/// A single particle entity component.
#[derive(Component, Debug)]
pub struct Particle {
    /// Screen x position.
    pub x: f32,
    /// Screen y position.
    pub y: f32,
    /// Velocity x in pixels per second.
    pub vx: f32,
    /// Velocity y in pixels per second.
    pub vy: f32,
    /// Seconds remaining.
    pub lifetime: f32,
    /// Initial lifetime for ratio calculations.
    pub max_lifetime: f32,
    /// RGBA color.
    pub color: ParticleColor,
    /// Size in pixels (square sprite).
    pub size: f32,
    /// Particle type for behavior.
    pub particle_type: ParticleType,
}

impl Particle {
    /// Returns true if the particle is still alive.
    fn is_alive(&self) -> bool {
        self.lifetime > 0.0
    }

    /// Returns lifetime as a ratio (1.0 = full, 0.0 = expired).
    fn lifetime_ratio(&self) -> f32 {
        if self.max_lifetime <= 0.0 {
            return 0.0;
        }
        self.lifetime / self.max_lifetime
    }

    /// Returns the current faded color based on remaining lifetime.
    fn current_color(&self) -> ParticleColor {
        let t = 1.0 - self.lifetime_ratio();
        match self.particle_type {
            ParticleType::Sparkle | ParticleType::Spark => {
                // Fade from white to golden
                ParticleColor::white().lerp(ParticleColor::golden(), t)
            }
            ParticleType::Flash => {
                // Bright red fades out
                let red = ParticleColor::red();
                ParticleColor(red.0, red.1, red.2, red.fade_alpha(1.0 - t))
            }
            ParticleType::Dust => {
                // Grey fades out
                let grey = ParticleColor::grey();
                ParticleColor(grey.0, grey.1, grey.2, grey.fade_alpha(1.0 - t * 0.7))
            }
        }
    }
}

// ─── Constants ────────────────────────────────────────────────────────────────

/// Maximum number of active particles (cap for mobile performance).
const MAX_ACTIVE_PARTICLES: usize = 200;

/// Number of checkpoint sparkle particles.
const CHECKPOINT_SPARKLE_COUNT: usize = 11;
/// Number of trap flash ring particles.
const TRAP_FLASH_COUNT: usize = 20;
/// Number of movement dust particles.
const MOVEMENT_DUST_COUNT: usize = 4;

// ─── Particle Spawner ─────────────────────────────────────────────────────────

/// Resource for spawning particles into the world.
/// Call `spawn_dust`, `spawn_checkpoint_sparkle`, or `spawn_trap_flash`
/// from any system to create particle bursts.
#[derive(Resource)]
pub struct ParticleSpawner {
    pub dust_color: ParticleColor,
    pub sparkle_color: ParticleColor,
    pub flash_color: ParticleColor,
}

impl Default for ParticleSpawner {
    fn default() -> Self {
        Self::new()
    }
}

impl ParticleSpawner {
    pub fn new() -> Self {
        Self {
            dust_color: ParticleColor::grey(),
            sparkle_color: ParticleColor::golden(),
            flash_color: ParticleColor::red(),
        }
    }

    /// Spawn movement dust particles at screen position (sx, sy).
    pub fn spawn_dust(&self, commands: &mut Commands, sx: f32, sy: f32) {
        for i in 0..MOVEMENT_DUST_COUNT {
            let angle = std::f32::consts::PI + (i as f32 - 2.0) * 0.5;
            let speed = 15.0 + (i as f32 % 2.0) * 10.0;
            let vx = angle.cos() * speed;
            let vy = angle.sin() * speed - 10.0; // slight upward bias
            let lifetime = 0.3 + (i as f32 % 2.0) * 0.1;
            let particle = Particle {
                x: sx,
                y: sy,
                vx,
                vy,
                lifetime,
                max_lifetime: lifetime,
                color: ParticleColor::grey(),
                size: 3.0,
                particle_type: ParticleType::Dust,
            };
            commands.spawn((
                particle,
                Transform::from_translation(Vec3::new(sx, sy, 2000.0)),
                Sprite::from_color(
                    Color::WHITE,
                    Vec2::splat(3.0),
                ),
            ));
        }
    }

    /// Spawn checkpoint sparkle particles at screen position (sx, sy).
    pub fn spawn_checkpoint_sparkle(&self, commands: &mut Commands, sx: f32, sy: f32) {
        // Burst of 11 golden sparkles rising upward
        for i in 0..CHECKPOINT_SPARKLE_COUNT {
            let angle = -std::f32::consts::FRAC_PI_2 + (i as f32 - 4.0) * 0.3;
            let speed = 60.0 + (i as f32 % 3.0) * 20.0;
            let vx = angle.cos() * speed;
            let vy = angle.sin() * speed; // negative = upward
            let lifetime = 0.5 + (i as f32 % 3.0) * 0.15;
            let color = if i % 2 == 0 {
                ParticleColor::white()
            } else {
                ParticleColor::golden()
            };
            let particle = Particle {
                x: sx,
                y: sy,
                vx,
                vy,
                lifetime,
                max_lifetime: lifetime,
                color,
                size: 4.0,
                particle_type: ParticleType::Sparkle,
            };
            commands.spawn((
                particle,
                Transform::from_translation(Vec3::new(sx, sy, 2000.0)),
                Sprite::from_color(
                    color.to_bevy_color(),
                    Vec2::splat(4.0),
                ),
            ));
        }
        // Extra 3 golden sparkles for bigger burst
        for i in 0..3 {
            let angle = -std::f32::consts::FRAC_PI_2 + (i as f32 - 1.0) * 0.4;
            let speed = 80.0 + i as f32 * 15.0;
            let vx = angle.cos() * speed;
            let vy = angle.sin() * speed;
            let lifetime = 0.6 + i as f32 * 0.1;
            let color = ParticleColor::golden();
            let particle = Particle {
                x: sx,
                y: sy,
                vx,
                vy,
                lifetime,
                max_lifetime: lifetime,
                color,
                size: 4.0,
                particle_type: ParticleType::Sparkle,
            };
            commands.spawn((
                particle,
                Transform::from_translation(Vec3::new(sx, sy, 2000.0)),
                Sprite::from_color(
                    color.to_bevy_color(),
                    Vec2::splat(4.0),
                ),
            ));
        }
    }

    /// Spawn trap flash particles at screen position (sx, sy).
    pub fn spawn_trap_flash(&self, commands: &mut Commands, sx: f32, sy: f32) {
        for i in 0..TRAP_FLASH_COUNT {
            let angle = (i as f32 / TRAP_FLASH_COUNT as f32) * std::f32::consts::TAU;
            let speed = 80.0;
            let vx = angle.cos() * speed;
            let vy = angle.sin() * speed;
            let lifetime = 0.4;
            let particle = Particle {
                x: sx,
                y: sy,
                vx,
                vy,
                lifetime,
                max_lifetime: lifetime,
                color: ParticleColor::red(),
                size: 6.0,
                particle_type: ParticleType::Flash,
            };
            commands.spawn((
                particle,
                Transform::from_translation(Vec3::new(sx, sy, 2000.0)),
                Sprite::from_color(
                    ParticleColor::red().to_bevy_color(),
                    Vec2::splat(6.0),
                ),
            ));
        }
    }

    /// Spawn generic sparkle at screen position.
    pub fn spawn_sparkle(&self, commands: &mut Commands, sx: f32, sy: f32) {
        for i in 0..4 {
            let angle = -std::f32::consts::FRAC_PI_2 + (i as f32 - 1.5) * 0.5;
            let speed = 50.0 + i as f32 * 10.0;
            let vx = angle.cos() * speed;
            let vy = angle.sin() * speed;
            let lifetime = 0.4 + i as f32 * 0.1;
            let particle = Particle {
                x: sx,
                y: sy,
                vx,
                vy,
                lifetime,
                max_lifetime: lifetime,
                color: ParticleColor::golden(),
                size: 3.0,
                particle_type: ParticleType::Spark,
            };
            commands.spawn((
                particle,
                Transform::from_translation(Vec3::new(sx, sy, 2000.0)),
                Sprite::from_color(
                    ParticleColor::golden().to_bevy_color(),
                    Vec2::splat(3.0),
                ),
            ));
        }
    }
}

// ─── Systems ─────────────────────────────────────────────────────────────────

/// Advances all particles by dt seconds. Removes expired particles.
/// Runs in PostUpdate so sprite positions are synced after movement.
pub fn particle_tick_system(
    mut commands: Commands,
    mut particles: Query<(Entity, &mut Particle)>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    let mut despawn_count = 0;

    for (entity, mut particle) in &mut particles {
        if particle.lifetime <= 0.0 {
            continue;
        }

        particle.lifetime -= dt;
        particle.x += particle.vx * dt;
        particle.y += particle.vy * dt;

        // Apply gravity/behavior per particle type
        match particle.particle_type {
            ParticleType::Sparkle | ParticleType::Spark => {
                // Gentle upward burst slows down (positive vy = downward pull)
                particle.vy += 40.0 * dt;
            }
            ParticleType::Dust => {
                // Slight rise then fall
                particle.vy += 20.0 * dt;
            }
            ParticleType::Flash => {
                // No gravity — ring expands outward
            }
        }

        if particle.lifetime <= 0.0 {
            commands.entity(entity).despawn();
            despawn_count += 1;
        }
    }

    if despawn_count > 0 {
        log::debug!("[Particles] Despawned {} expired particles", despawn_count);
    }
}

/// Syncs Transform and Sprite color from Particle state.
/// Runs in PostUpdate after particle_tick_system.
pub fn particle_sync_system(
    mut query: Query<(&Particle, &mut Transform, &mut Sprite)>,
) {
    for (particle, mut transform, mut sprite) in &mut query {
        transform.translation.x = particle.x;
        transform.translation.y = particle.y;
        transform.translation.z = 2000.0; // particles always in front

        // Update tint color based on current fade state
        let color = particle.current_color();
        sprite.color = color.to_bevy_color();
    }
}

// ─── Plugin ───────────────────────────────────────────────────────────────────

/// Plugin that registers the particle system.
///
/// On `PreStartup`: creates the particle texture atlas (white 4x4 pixel).
/// On `PostUpdate`: runs particle_tick_system and particle_sync_system.
pub struct ParticlePlugin;

impl ParticlePlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Plugin for ParticlePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, (
            particle_tick_system,
            particle_sync_system,
        ))
        .insert_resource(ParticleSpawner::new());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_particle_color_golden() {
        let c = ParticleColor::golden();
        assert_eq!(c.0, 255);
        assert_eq!(c.1, 215);
        assert_eq!(c.2, 80);
    }

    #[test]
    fn test_particle_color_white() {
        let c = ParticleColor::white();
        assert_eq!(c.0, 255);
        assert_eq!(c.1, 255);
        assert_eq!(c.2, 255);
    }

    #[test]
    fn test_particle_color_grey() {
        let c = ParticleColor::grey();
        assert_eq!(c.0, 180);
        assert_eq!(c.1, 180);
        assert_eq!(c.2, 180);
    }

    #[test]
    fn test_particle_color_red() {
        let c = ParticleColor::red();
        assert_eq!(c.0, 255);
        assert_eq!(c.1, 60);
        assert_eq!(c.2, 60);
    }

    #[test]
    fn test_particle_color_lerp() {
        let white = ParticleColor::white();
        let golden = ParticleColor::golden();
        let lerped = white.lerp(golden, 0.5);
        assert_eq!(lerped.0, 255); // (255+255)/2 = 255
        assert_eq!(lerped.1, 235); // (255+215)/2 = 235
        assert_eq!(lerped.2, 167); // (255+80)/2 = 167
    }

    #[test]
    fn test_particle_color_fade_alpha() {
        let c = ParticleColor::grey();
        assert_eq!(c.fade_alpha(1.0), 200);
        assert_eq!(c.fade_alpha(0.5), 100);
        assert_eq!(c.fade_alpha(0.0), 0);
    }

    #[test]
    fn test_particle_is_alive() {
        let p = Particle {
            x: 0.0,
            y: 0.0,
            vx: 0.0,
            vy: 0.0,
            lifetime: 0.1,
            max_lifetime: 0.5,
            color: ParticleColor::grey(),
            size: 3.0,
            particle_type: ParticleType::Dust,
        };
        assert!(p.is_alive());
    }

    #[test]
    fn test_particle_lifetime_ratio() {
        let p = Particle {
            x: 0.0,
            y: 0.0,
            vx: 0.0,
            vy: 0.0,
            lifetime: 0.25,
            max_lifetime: 0.5,
            color: ParticleColor::grey(),
            size: 3.0,
            particle_type: ParticleType::Dust,
        };
        assert!((p.lifetime_ratio() - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_particle_current_color_dust_fades() {
        let p = Particle {
            x: 0.0,
            y: 0.0,
            vx: 0.0,
            vy: 0.0,
            lifetime: 0.0, // expired
            max_lifetime: 0.5,
            color: ParticleColor::grey(),
            size: 3.0,
            particle_type: ParticleType::Dust,
        };
        let faded = p.current_color();
        assert!(faded.3 < p.color.3, "alpha should decrease as particle ages");
    }

    #[test]
    fn test_particle_spawner_default() {
        let spawner = ParticleSpawner::new();
        assert_eq!(spawner.dust_color, ParticleColor::grey());
        assert_eq!(spawner.sparkle_color, ParticleColor::golden());
        assert_eq!(spawner.flash_color, ParticleColor::red());
    }

    #[test]
    fn test_particle_color_to_bevy_color() {
        // Verify color conversion doesn't panic and produces a valid Bevy Color
        let c = ParticleColor::white();
        let bev = c.to_bevy_color();
        // In Bevy 0.18, Color is an enum — just verify it's constructable
        let _ = format!("{:?}", bev);
    }

    #[test]
    fn test_particle_color_red_to_bevy_color() {
        let c = ParticleColor::red();
        let bev = c.to_bevy_color();
        let _ = format!("{:?}", bev);
    }

    #[test]
    fn test_particle_sparkle_lifetime_ratio_zero_max() {
        let p = Particle {
            x: 0.0,
            y: 0.0,
            vx: 0.0,
            vy: 0.0,
            lifetime: 0.1,
            max_lifetime: 0.0,
            color: ParticleColor::grey(),
            size: 3.0,
            particle_type: ParticleType::Sparkle,
        };
        assert_eq!(p.lifetime_ratio(), 0.0);
    }
}
