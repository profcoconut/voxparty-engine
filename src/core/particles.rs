//! Lightweight particle system for visual juice.
//!
//! Particles are short-lived (0.3–1s), spawned at fixed positions,
//! rendered as small colored squares that fade out.
//!
//! Types:
//! - **Checkpoint sparkle**: golden/white particles burst upward from checkpoint tile.
//! - **Trap flash**: red ring expands and fades on trap trigger.
//! - **Movement dust**: small grey puff at player's feet when they move.

use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;

/// RGBA color for a particle.
#[derive(Debug, Clone, Copy)]
struct ParticleColor {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl ParticleColor {
    fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    fn golden() -> Self {
        Self { r: 255, g: 215, b: 80, a: 255 }
    }

    fn white() -> Self {
        Self { r: 255, g: 255, b: 255, a: 255 }
    }

    fn grey() -> Self {
        Self { r: 180, g: 180, b: 180, a: 200 }
    }

    fn red() -> Self {
        Self { r: 255, g: 60, b: 60, a: 255 }
    }

    /// Interpolate between two colors.
    fn lerp(self, other: ParticleColor, t: f32) -> ParticleColor {
        let t = t.clamp(0.0, 1.0);
        ParticleColor {
            r: (self.r as f32 + (other.r as f32 - self.r as f32) * t) as u8,
            g: (self.g as f32 + (other.g as f32 - self.g as f32) * t) as u8,
            b: (self.b as f32 + (other.b as f32 - self.b as f32) * t) as u8,
            a: (self.a as f32 + (other.a as f32 - self.a as f32) * t) as u8,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParticleType {
    /// Golden/white sparkle burst — rises upward from checkpoint.
    CheckpointSparkle,
    /// Red ring that expands and fades — triggered on trap.
    TrapFlash,
    /// Small grey puff at player's feet when they move.
    MovementDust,
}

/// A single particle instance.
#[derive(Debug, Clone)]
struct Particle {
    x: f32,
    y: f32,
    /// Velocity in pixels per second.
    vx: f32,
    vy: f32,
    /// Lifetime in seconds. 0 = expired.
    lifetime: f32,
    max_lifetime: f32,
    color: ParticleColor,
    /// Size in pixels (square).
    size: i32,
    particle_type: ParticleType,
}

impl Particle {
    /// Returns true if this particle is still alive.
    fn is_alive(&self) -> bool {
        self.lifetime > 0.0
    }

    /// Returns the RGBA color blended with fade-out based on remaining lifetime.
    fn current_color(&self) -> ParticleColor {
        let t = 1.0 - (self.lifetime / self.max_lifetime);
        match self.particle_type {
            ParticleType::CheckpointSparkle => {
                // Fade from white to golden
                let golden = ParticleColor::golden();
                let white = ParticleColor::white();
                white.lerp(golden, t)
            }
            ParticleType::TrapFlash => {
                // Starts bright red, fades to transparent
                let red = ParticleColor::red();
                ParticleColor::new(red.r, red.g, red.b, (red.a as f32 * (1.0 - t)) as u8)
            }
            ParticleType::MovementDust => {
                // Grey fades out
                let grey = ParticleColor::grey();
                ParticleColor::new(grey.r, grey.g, grey.b, (grey.a as f32 * (1.0 - t * 0.7)) as u8)
            }
        }
    }

    /// Advance by `dt` seconds.
    fn tick(&mut self, dt: f32) {
        if self.lifetime <= 0.0 {
            return;
        }
        self.lifetime -= dt;
        self.x += self.vx * dt;
        self.y += self.vy * dt;

        // Gravity for dust and sparkles (slight downward pull)
        match self.particle_type {
            ParticleType::CheckpointSparkle => {
                self.vy += 40.0 * dt; // gentle upward burst slows down
            }
            ParticleType::MovementDust => {
                self.vy += 20.0 * dt; // slight rise then fall
            }
            ParticleType::TrapFlash => {
                // No gravity — ring expands outward
            }
        }
    }
}

/// Maximum number of active particles to prevent mobile performance issues.
const MAX_PARTICLES: usize = 50;

/// Particle system — manages spawn, tick, and draw of all active particles.
#[derive(Debug)]
pub struct ParticleSystem {
    particles: Vec<Particle>,
}

impl ParticleSystem {
    pub fn new() -> Self {
        Self {
            particles: Vec::with_capacity(256),
        }
    }

    /// Spawn a particle at the given screen position.
    /// Particle count is capped at MAX_PARTICLES to prevent mobile performance issues.
    pub fn spawn(&mut self, x: f32, y: f32, ptype: ParticleType) {
        // If we're at or above the cap, don't spawn more particles
        if self.particles.len() >= MAX_PARTICLES {
            return;
        }

        match ptype {
            ParticleType::CheckpointSparkle => {
                // Burst of ~8 golden sparkles rising upward
                for i in 0..8 {
                    // Check cap before each particle
                    if self.particles.len() >= MAX_PARTICLES {
                        break;
                    }
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
                    self.particles.push(Particle {
                        x,
                        y,
                        vx,
                        vy,
                        lifetime,
                        max_lifetime: lifetime,
                        color,
                        size: 4,
                        particle_type: ptype,
                    });
                }
            }
            ParticleType::TrapFlash => {
                // Single large ring that expands outward
                // We'll use multiple particles in a ring pattern
                let num_particles = 16;
                for i in 0..num_particles {
                    // Check cap before each particle
                    if self.particles.len() >= MAX_PARTICLES {
                        break;
                    }
                    let angle = (i as f32 / num_particles as f32) * std::f32::consts::TAU;
                    let speed = 80.0;
                    let vx = angle.cos() * speed;
                    let vy = angle.sin() * speed;
                    let lifetime = 0.4;
                    self.particles.push(Particle {
                        x,
                        y,
                        vx,
                        vy,
                        lifetime,
                        max_lifetime: lifetime,
                        color: ParticleColor::red(),
                        size: 6,
                        particle_type: ptype,
                    });
                }
            }
            ParticleType::MovementDust => {
                // Small puff of 3-4 grey particles at feet
                for i in 0..4 {
                    // Check cap before each particle
                    if self.particles.len() >= MAX_PARTICLES {
                        break;
                    }
                    let angle = std::f32::consts::PI + (i as f32 - 2.0) * 0.5;
                    let speed = 15.0 + (i as f32 % 2.0) * 10.0;
                    let vx = angle.cos() * speed;
                    let vy = angle.sin() * speed - 10.0; // slight upward bias
                    let lifetime = 0.3 + (i as f32 % 2.0) * 0.1;
                    self.particles.push(Particle {
                        x,
                        y,
                        vx,
                        vy,
                        lifetime,
                        max_lifetime: lifetime,
                        color: ParticleColor::grey(),
                        size: 3,
                        particle_type: ptype,
                    });
                }
            }
        }
    }

    /// Advance all particles by `dt` seconds. Removes dead particles.
    pub fn tick(&mut self, dt: f32) {
        for p in &mut self.particles {
            p.tick(dt);
        }
        self.particles.retain(|p| p.is_alive());
    }

    /// Draw all particles onto the canvas at their screen position.
    pub fn draw(&self, canvas: &mut Canvas<Window>) {
        for p in &self.particles {
            let color = p.current_color();
            // Particle position is already in screen coordinates (camera-adjusted at spawn time)
            let rect = Rect::new(p.x as i32, p.y as i32, p.size as u32, p.size as u32);
            canvas.set_draw_color(sdl2::pixels::Color::RGBA(color.r, color.g, color.b, color.a));
            let _ = canvas.fill_rect(rect);
        }
    }

    /// Returns the number of active particles.
    pub fn len(&self) -> usize {
        self.particles.len()
    }

    /// Returns true if there are no active particles.
    pub fn is_empty(&self) -> bool {
        self.particles.is_empty()
    }
}

impl Default for ParticleSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_particle_system_is_empty() {
        let ps = ParticleSystem::new();
        assert!(ps.is_empty());
        assert_eq!(ps.len(), 0);
    }

    #[test]
    fn test_spawn_checkpoint_adds_particles() {
        let mut ps = ParticleSystem::new();
        ps.spawn(100.0, 200.0, ParticleType::CheckpointSparkle);
        assert_eq!(ps.len(), 8, "checkpoint sparkle should spawn 8 particles");
    }

    #[test]
    fn test_spawn_trap_flash_adds_particles() {
        let mut ps = ParticleSystem::new();
        ps.spawn(100.0, 200.0, ParticleType::TrapFlash);
        assert_eq!(ps.len(), 16, "trap flash should spawn 16 particles");
    }

    #[test]
    fn test_spawn_movement_dust_adds_particles() {
        let mut ps = ParticleSystem::new();
        ps.spawn(100.0, 200.0, ParticleType::MovementDust);
        assert_eq!(ps.len(), 4, "movement dust should spawn 4 particles");
    }

    #[test]
    fn test_particle_expires_after_tick() {
        let mut ps = ParticleSystem::new();
        ps.spawn(100.0, 200.0, ParticleType::MovementDust);
        assert_eq!(ps.len(), 4);
        // Tick with dt larger than particle lifetime
        ps.tick(1.0);
        assert!(ps.is_empty());
    }

    #[test]
    fn test_particle_color_fade() {
        let p = Particle {
            x: 0.0,
            y: 0.0,
            vx: 0.0,
            vy: 0.0,
            lifetime: 0.25,
            max_lifetime: 0.5,
            color: ParticleColor::grey(),
            size: 3,
            particle_type: ParticleType::MovementDust,
        };
        // Half lifetime remaining → color should be partially faded
        let faded = p.current_color();
        assert!(faded.a < p.color.a, "alpha should decrease as particle ages");
    }
}
