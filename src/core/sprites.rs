use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct SpriteFrame {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

#[derive(Debug, Clone)]
pub struct SpriteAnimation {
    pub name: String,
    pub frames: Vec<SpriteFrame>,
    pub fps: f32,
}

#[derive(Debug, Clone)]
pub struct SpriteSheet {
    pub name: String,
    /// Map from frame name → frame rect in the sprite sheet image
    pub frames: HashMap<String, SpriteFrame>,
    /// Map from animation name → animation definition
    pub animations: HashMap<String, SpriteAnimation>,
}

impl SpriteSheet {
    /// Load a sprite sheet from a JSON descriptor.
    /// Expected JSON format:
    /// {
    ///   "meta": { "image": "filename.png", "tile_w": 64, "tile_h": 64 },
    ///   "frames": { "name": { "x": 0, "y": 0, "w": 64, "h": 64 }, ... },
    ///   "animations": { "anim_name": { "frames": ["frame1", "frame2"], "fps": 8 }, ... }
    /// }
    pub fn from_json(json_str: &str) -> Self {
        #[derive(serde::Deserialize)]
        struct Meta {
            image: String,
            #[allow(dead_code)]
            tile_w: u32,
            #[allow(dead_code)]
            tile_h: u32,
        }
        #[derive(serde::Deserialize)]
        struct FrameDef {
            x: u32,
            y: u32,
            w: u32,
            h: u32,
        }
        #[derive(serde::Deserialize)]
        struct AnimDef {
            frames: Vec<String>,
            fps: f32,
        }
        #[derive(serde::Deserialize)]
        struct SheetJSON {
            meta: Meta,
            frames: HashMap<String, FrameDef>,
            animations: HashMap<String, AnimDef>,
        }

        let sheet: SheetJSON = serde_json::from_str(json_str)
            .expect("Invalid sprite sheet JSON");

        let mut frames = HashMap::new();
        for (name, def) in sheet.frames {
            frames.insert(name, SpriteFrame { x: def.x, y: def.y, w: def.w, h: def.h });
        }

        let mut animations = HashMap::new();
        for (name, def) in sheet.animations {
            let anim_frames: Vec<SpriteFrame> = def
                .frames
                .iter()
                .filter_map(|f| frames.get(f).cloned())
                .collect();
            animations.insert(
                name.clone(),
                SpriteAnimation {
                    name,
                    frames: anim_frames,
                    fps: def.fps,
                },
            );
        }

        Self {
            name: sheet.meta.image,
            frames,
            animations,
        }
    }
}

/// Per-entity animation state machine.
pub struct AnimPlayer {
    current_anim: Option<String>,
    frame_index: usize,
    frame_timer: f32,
    /// Cached current frame (updated by tick(), used by render)
    current_frame: Option<SpriteFrame>,
}

impl AnimPlayer {
    pub fn new() -> Self {
        Self {
            current_anim: None,
            frame_index: 0,
            frame_timer: 0.0,
            current_frame: None,
        }
    }

    /// Get the current animation frame (from last tick) without advancing.
    pub fn current_frame(&self) -> Option<&SpriteFrame> {
        self.current_frame.as_ref()
    }

    /// Start playing an animation. `force` restarts even if already playing.
    pub fn play(&mut self, anim_name: &str, _sheet: &SpriteSheet, force: bool) {
        if !force && self.current_anim.as_deref() == Some(anim_name) {
            return;
        }
        self.current_anim = Some(anim_name.to_string());
        self.frame_index = 0;
        self.frame_timer = 0.0;
    }

    /// Explicitly advance the animation by one frame.
    /// Called after play() to step to the next frame immediately.
    pub fn advance(&mut self, sheet: &SpriteSheet) {
        if self.current_anim.is_none() {
            return;
        }
        let anim_name = self.current_anim.as_ref().unwrap();
        let anim = match sheet.animations.get(anim_name) {
            Some(a) => a,
            None => return,
        };
        if anim.frames.is_empty() {
            return;
        }
        self.frame_index = (self.frame_index + 1) % anim.frames.len().max(1);
        self.current_frame = Some(anim.frames[self.frame_index].clone());
    }

    /// Advance animation by `dt` seconds. Returns the current frame, if any.
    /// Also stores the frame in `self.current_frame` for retrieval via `current_frame()`.
    pub fn tick<'a>(&mut self, dt: f32, sheet: &'a SpriteSheet) -> Option<&'a SpriteFrame> {
        let anim_name = self.current_anim.as_ref()?;
        let anim = sheet.animations.get(anim_name)?;
        if anim.frames.is_empty() {
            self.current_frame = None;
            return None;
        }

        self.frame_timer += dt;
        let frame_duration = 1.0 / anim.fps.max(1.0);
        while self.frame_timer >= frame_duration {
            self.frame_timer -= frame_duration;
            self.frame_index = (self.frame_index + 1) % anim.frames.len().max(1);
        }
        self.current_frame = Some(anim.frames[self.frame_index].clone());
        Some(&anim.frames[self.frame_index])
    }
}

impl Default for AnimPlayer {
    fn default() -> Self { Self::new() }
}
