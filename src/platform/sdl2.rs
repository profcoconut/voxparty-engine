use sdl2::video::WindowContext;
use sdl2::EventPump;
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;
use std::collections::HashMap;

/// Raw sprite data stored with 'static lifetime (owned heap allocation).
pub struct RawSprite {
    pub data: Box<[u8]>,
    pub width: u32,
    pub height: u32,
    pub format: PixelFormatEnum,
}

pub struct Platform {
    pub event_pump: EventPump,
    pub canvas: sdl2::render::Canvas<sdl2::video::Window>,
    pub texture_creator: sdl2::render::TextureCreator<WindowContext>,
    pub sprites: HashMap<String, RawSprite>,
    pub target_fps: u32,
}

impl Platform {
    pub fn new(title: &str, width: u32, height: u32) -> Self {
        let sdl = sdl2::init().expect("SDL2 init failed");
        let video = sdl.video().expect("SDL2 video init failed");

        let window = video
            .window(title, width, height)
            .position_centered()
            .allow_highdpi()
            .build()
            .expect("Window creation failed");

        let mut canvas = window
            .into_canvas()
            .accelerated()
            .present_vsync()
            .build()
            .expect("Canvas creation failed");

        canvas.set_blend_mode(sdl2::render::BlendMode::Blend);

        let event_pump = sdl.event_pump().expect("Event pump creation failed");
        let texture_creator = canvas.texture_creator();

        Self {
            event_pump,
            canvas,
            texture_creator,
            sprites: HashMap::new(),
            target_fps: 60,
        }
    }

    pub fn clear(&mut self, r: u8, g: u8, b: u8, a: u8) {
        self.canvas.set_draw_color(sdl2::pixels::Color::RGBA(r, g, b, a));
        self.canvas.clear();
    }

    pub fn present(&mut self) {
        self.canvas.present();
    }

    pub fn delay(&self, millis: u32) {
        std::thread::sleep(std::time::Duration::from_millis(millis.into()));
    }

    pub fn load_sprite(&mut self, name: &str, path: &str) -> Result<(), String> {
        let img = image::open(path).map_err(|e| e.to_string())?;
        let rgba = img.to_rgba8();
        let (w, h) = rgba.dimensions();
        let data: Box<[u8]> = rgba.into_raw().into_boxed_slice();
        self.sprites.insert(
            name.to_string(),
            RawSprite {
                data,
                width: w,
                height: h,
                format: PixelFormatEnum::RGBA32,
            },
        );
        Ok(())
    }

    /// Blit a sprite by name at the given destination rect (in screen pixels).
    /// Creates a transient texture for the blit.
    pub fn blit_sprite(&mut self, name: &str, dst: Rect, src: Option<Rect>) -> Result<(), String> {
        let sprite = self.sprites.get(name).ok_or_else(|| format!("Sprite not found: {}", name))?;

        // Build surface from owned sprite data (Surface borrows our Box<[u8]> which is 'static)
        let mut data_clone = sprite.data.clone();
        let mut surface = sdl2::surface::Surface::from_data(
            &mut data_clone,
            sprite.width,
            sprite.height,
            sprite.width * 4,
            sprite.format,
        )
        .map_err(|e| e.to_string())?;

        let texture = self
            .texture_creator
            .create_texture_from_surface(&mut surface)
            .map_err(|e| e.to_string())?;

        self.canvas.copy(&texture, src, dst).map_err(|e| e.to_string())
    }
}
