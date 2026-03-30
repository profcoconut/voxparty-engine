use rodio::{Sink, Source, OutputStream, OutputStreamHandle};
use std::collections::HashMap;
use std::fs::File;

use crate::game::episode::Episode;

/// Simple audio manager using rodio.
pub struct AudioManager {
    _stream: OutputStream,
    _stream_handle: OutputStreamHandle,
    music_sink: Option<Sink>,
    /// Maps sound name → WAV/MP3 file contents
    sfx_data: HashMap<String, Vec<u8>>,
    /// Master volume (0.0 to 1.0)
    volume: f32,
}

impl AudioManager {
    pub fn new() -> Self {
        let (stream, stream_handle) = OutputStream::try_default()
            .expect("Failed to open audio output");
        Self {
            _stream: stream,
            _stream_handle: stream_handle,
            music_sink: None,
            sfx_data: HashMap::new(),
            volume: 1.0,
        }
    }

    /// Set master volume (0.0 to 1.0). Affects both music and SFX.
    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        // Apply to currently playing music sink
        if let Some(ref sink) = self.music_sink {
            sink.set_volume(self.volume);
        }
    }

    /// Get current master volume.
    pub fn volume(&self) -> f32 {
        self.volume
    }

    /// Load a sound effect file into memory.
    pub fn load_sfx(&mut self, name: &str, path: &str) {
        if let Ok(data) = std::fs::read(path) {
            self.sfx_data.insert(name.to_string(), data);
        }
    }

    /// Play a sound effect by name.
    /// Falls back to synthesized SFX when no file data is available.
    pub fn play_sfx(&mut self, name: &str) {
        // Try file-based SFX first
        if let Some(data) = self.sfx_data.get(name) {
            let cursor = std::io::Cursor::new(data.clone());
            if let Ok(decoder) = rodio::Decoder::new(cursor) {
                if let Ok(sink) = Sink::try_new(&self._stream_handle) {
                    sink.append(decoder.amplify(self.volume));
                    return;
                }
            }
        }
        // Fall back to synthesized SFX (volume applied in each synth method)
        match name {
            "jump" | "move" => self.play_synth_move(),
            "eliminate" | "trap" => self.play_synth_trap(),
            "checkpoint" => self.play_synth_checkpoint(),
            "victory" | "win" => self.play_synth_victory(),
            _ => {}
        }
    }

    // -------------------------------------------------------------------------
    // Synthesized SFX methods
    // -------------------------------------------------------------------------

    /// Short blip for movement — 80ms sine wave at 880Hz with quick decay.
    fn play_synth_move(&mut self) {
        use rodio::Source;
        if let Ok(sink) = Sink::try_new(&self._stream_handle) {
            let tone = rodio::source::SineWave::new(880.0)
                .take_duration(std::time::Duration::from_millis(80))
                .fade_in(std::time::Duration::from_millis(5))
                .amplify(0.4 * self.volume);
            sink.append(tone);
        }
    }

    /// Descending dissonant tone for trap/elimination.
    /// Uses two detuned sine waves sweeping down for harsh buzzer effect.
    fn play_synth_trap(&mut self) {
        use rodio::Source;
        if let Ok(sink) = Sink::try_new(&self._stream_handle) {
            // Descending sine with dissonant second tone
            let descend1 = rodio::source::SineWave::new(400.0)
                .take_duration(std::time::Duration::from_millis(350))
                .fade_in(std::time::Duration::from_millis(10))
                .amplify(0.35 * self.volume);
            let descend2 = rodio::source::SineWave::new(420.0)
                .take_duration(std::time::Duration::from_millis(350))
                .fade_in(std::time::Duration::from_millis(10))
                .amplify(0.25 * self.volume);
            // Low thud underneath
            let thud = rodio::source::SineWave::new(60.0)
                .take_duration(std::time::Duration::from_millis(200))
                .amplify(0.4 * self.volume);
            sink.append(descend1.mix(descend2).mix(thud));
        }
    }

    /// Ascending chime for checkpoint — three rising notes (C5, E5, G5).
    fn play_synth_checkpoint(&mut self) {
        use rodio::Source;
        if let Ok(sink) = Sink::try_new(&self._stream_handle) {
            let c5 = rodio::source::SineWave::new(523.0)
                .take_duration(std::time::Duration::from_millis(120))
                .fade_in(std::time::Duration::from_millis(10))
                .amplify(0.4 * self.volume);
            let e5 = rodio::source::SineWave::new(659.0)
                .take_duration(std::time::Duration::from_millis(120))
                .fade_in(std::time::Duration::from_millis(10))
                .amplify(0.4 * self.volume);
            let g5 = rodio::source::SineWave::new(784.0)
                .take_duration(std::time::Duration::from_millis(180))
                .fade_in(std::time::Duration::from_millis(10))
                .amplify(0.4 * self.volume);
            sink.append(c5);
            sink.sleep_until_end();
            sink.append(e5);
            sink.sleep_until_end();
            sink.append(g5);
        }
    }

    /// Short C major arpeggio fanfare for reaching the goal.
    /// C4 (262Hz) for 100ms → E4 (330Hz) for 100ms → G4 (392Hz) for 100ms → C5 (523Hz) for 300ms.
    /// Total duration: ~600ms. Plays once, not looped.
    pub fn play_victory_jingle(&mut self) {
        use rodio::Source;
        if let Ok(sink) = Sink::try_new(&self._stream_handle) {
            let c4 = rodio::source::SineWave::new(262.0)
                .take_duration(std::time::Duration::from_millis(100))
                .fade_in(std::time::Duration::from_millis(10))
                .amplify(0.45 * self.volume);
            let e4 = rodio::source::SineWave::new(330.0)
                .take_duration(std::time::Duration::from_millis(100))
                .fade_in(std::time::Duration::from_millis(10))
                .amplify(0.45 * self.volume);
            let g4 = rodio::source::SineWave::new(392.0)
                .take_duration(std::time::Duration::from_millis(100))
                .fade_in(std::time::Duration::from_millis(10))
                .amplify(0.45 * self.volume);
            let c5 = rodio::source::SineWave::new(523.0)
                .take_duration(std::time::Duration::from_millis(300))
                .fade_in(std::time::Duration::from_millis(10))
                .amplify(0.5 * self.volume);
            sink.append(c4);
            sink.sleep_until_end();
            sink.append(e4);
            sink.sleep_until_end();
            sink.append(g4);
            sink.sleep_until_end();
            sink.append(c5);
        }
    }

    /// Victory fanfare — short major chord progression: C major → E major → G major → C octave.
    fn play_synth_victory(&mut self) {
        use rodio::Source;
        if let Ok(sink) = Sink::try_new(&self._stream_handle) {
            // C major chord (C4, E4, G4) — 200ms
            let c_chord = rodio::source::SineWave::new(261.63)
                .mix(rodio::source::SineWave::new(329.63).amplify(0.7))
                .mix(rodio::source::SineWave::new(392.0).amplify(0.6))
                .take_duration(std::time::Duration::from_millis(200))
                .fade_in(std::time::Duration::from_millis(15))
                .amplify(0.45 * self.volume);
            // E major (E4, G#4, B4)
            let e_chord = rodio::source::SineWave::new(329.63)
                .mix(rodio::source::SineWave::new(415.30).amplify(0.7))
                .mix(rodio::source::SineWave::new(493.88).amplify(0.6))
                .take_duration(std::time::Duration::from_millis(200))
                .fade_in(std::time::Duration::from_millis(15))
                .amplify(0.45 * self.volume);
            // G major (G4, B4, D5)
            let g_chord = rodio::source::SineWave::new(392.0)
                .mix(rodio::source::SineWave::new(493.88).amplify(0.7))
                .mix(rodio::source::SineWave::new(587.33).amplify(0.6))
                .take_duration(std::time::Duration::from_millis(200))
                .fade_in(std::time::Duration::from_millis(15))
                .amplify(0.45 * self.volume);
            // C octave (C5) — longer
            let c5 = rodio::source::SineWave::new(523.25)
                .mix(rodio::source::SineWave::new(659.25).amplify(0.5))
                .take_duration(std::time::Duration::from_millis(400))
                .fade_in(std::time::Duration::from_millis(15))
                .amplify(0.45 * self.volume);
            sink.append(c_chord);
            sink.sleep_until_end();
            sink.append(e_chord);
            sink.sleep_until_end();
            sink.append(g_chord);
            sink.sleep_until_end();
            sink.append(c5);
        }
    }

    /// Somber descending minor second for game over — C4 (262Hz) for 200ms then Bb3 (233Hz) for 400ms.
    /// Total duration ~600ms. Only plays once per game over transition.
    pub fn play_gameover_sound(&mut self) {
        use rodio::Source;
        if let Ok(sink) = Sink::try_new(&self._stream_handle) {
            // C4 at 262Hz for 200ms
            let c4 = rodio::source::SineWave::new(262.0)
                .take_duration(std::time::Duration::from_millis(200))
                .fade_in(std::time::Duration::from_millis(10))
                .amplify(0.4 * self.volume);
            // Bb3 at 233Hz for 400ms (slight decay feel)
            let bb3 = rodio::source::SineWave::new(233.0)
                .take_duration(std::time::Duration::from_millis(400))
                .fade_in(std::time::Duration::from_millis(10))
                .amplify(0.35 * self.volume);
            sink.append(c4);
            sink.sleep_until_end();
            sink.append(bb3);
        }
    }

    /// Play a synthesized step sound based on the tile type material.
    /// - grass types: Footstep sound — short, soft, high-frequency noise burst
    /// - stone types: Clank — short metallic click, mid frequency
    /// - ice types: Crunch — slightly longer noise, lower pitch than grass
    /// - lava types: Hiss — white noise burst with quick decay
    /// - bridge/wood types: Creak — low wooden thud
    pub fn play_synth_step(&mut self, tile_type: &str) {
        use rodio::Source;

        // Determine material from tile type string prefix
        let material = if tile_type.starts_with("grass") {
            "grass"
        } else if tile_type.starts_with("stone") {
            "stone"
        } else if tile_type.starts_with("ice") || tile_type.starts_with("snow") {
            "ice"
        } else if tile_type.starts_with("lava") {
            "lava"
        } else if tile_type.starts_with("bridge") || tile_type.starts_with("wood") {
            "bridge"
        } else {
            "default"
        };

        if let Ok(sink) = Sink::try_new(&self._stream_handle) {
            match material {
                "grass" => {
                    // Footstep: 800Hz sine + high-freq noise for 50ms
                    let tone = rodio::source::SineWave::new(800.0)
                        .take_duration(std::time::Duration::from_millis(50))
                        .fade_in(std::time::Duration::from_millis(5))
                        .amplify(0.3 * self.volume);
                    // Mix with a short noise burst (approximated with high-freq sine)
                    let noise = rodio::source::SineWave::new(2000.0)
                        .take_duration(std::time::Duration::from_millis(30))
                        .amplify(0.15 * self.volume);
                    sink.append(tone.mix(noise));
                }
                "stone" => {
                    // Clank: 300Hz sine wave with quick decay for metallic click
                    // Add harmonics to simulate metallic timbre
                    let fundamental = rodio::source::SineWave::new(300.0)
                        .take_duration(std::time::Duration::from_millis(30))
                        .fade_in(std::time::Duration::from_millis(2))
                        .amplify(0.3 * self.volume);
                    let harmonic = rodio::source::SineWave::new(600.0)
                        .take_duration(std::time::Duration::from_millis(20))
                        .amplify(0.15 * self.volume);
                    // Brief high-frequency click for attack transient
                    let click = rodio::source::SineWave::new(1200.0)
                        .take_duration(std::time::Duration::from_millis(8))
                        .amplify(0.25 * self.volume);
                    sink.append(fundamental.mix(harmonic).mix(click));
                }
                "ice" => {
                    // Crunch: 400Hz + noise for 80ms — lower pitch than grass
                    let tone = rodio::source::SineWave::new(400.0)
                        .take_duration(std::time::Duration::from_millis(80))
                        .fade_in(std::time::Duration::from_millis(10))
                        .amplify(0.25 * self.volume);
                    // Gritty noise component
                    let grit = rodio::source::SineWave::new(800.0)
                        .take_duration(std::time::Duration::from_millis(60))
                        .amplify(0.15 * self.volume);
                    sink.append(tone.mix(grit));
                }
                "lava" => {
                    // Hiss: white noise for 100ms with fast decay
                    // Approximated with broad-spectrum mix of high frequencies
                    let hiss1 = rodio::source::SineWave::new(3000.0)
                        .take_duration(std::time::Duration::from_millis(100))
                        .fade_in(std::time::Duration::from_millis(5))
                        .amplify(0.2 * self.volume);
                    let hiss2 = rodio::source::SineWave::new(4000.0)
                        .take_duration(std::time::Duration::from_millis(80))
                        .amplify(0.15 * self.volume);
                    let hiss3 = rodio::source::SineWave::new(2500.0)
                        .take_duration(std::time::Duration::from_millis(90))
                        .amplify(0.15 * self.volume);
                    sink.append(hiss1.mix(hiss2).mix(hiss3));
                }
                "bridge" => {
                    // Creak: low 150Hz sine wave for 80ms — wooden thud
                    // Mix with a slightly higher frequency for woody resonance
                    let low_tone = rodio::source::SineWave::new(150.0)
                        .take_duration(std::time::Duration::from_millis(80))
                        .fade_in(std::time::Duration::from_millis(10))
                        .amplify(0.35 * self.volume);
                    let resonance = rodio::source::SineWave::new(280.0)
                        .take_duration(std::time::Duration::from_millis(50))
                        .amplify(0.2 * self.volume);
                    sink.append(low_tone.mix(resonance));
                }
                _ => {
                    // Default footstep: soft mid-frequency tone
                    let default_tone = rodio::source::SineWave::new(500.0)
                        .take_duration(std::time::Duration::from_millis(40))
                        .fade_in(std::time::Duration::from_millis(5))
                        .amplify(0.25 * self.volume);
                    sink.append(default_tone);
                }
            }
        }
    }

    /// Start looping background music.
    pub fn play_music(&mut self, path: &str) {
        if let Ok(file) = File::open(path) {
            if let Ok(decoder) = rodio::Decoder::new(file) {
                let sink = match Sink::try_new(&self._stream_handle) {
                    Ok(sink) => sink,
                    Err(e) => {
                        log::warn!("Failed to create audio sink for music: {}", e);
                        return;
                    }
                };
                sink.set_volume(self.volume);
                sink.append(decoder.repeat_infinite());
                self.music_sink = Some(sink);
            }
        }
    }

    /// Stop background music.
    pub fn stop_music(&mut self) {
        if let Some(sink) = self.music_sink.take() {
            sink.stop();
        }
    }

    /// Play a simple synthesized sine wave tone as a placeholder for real music.
    /// This allows the game to produce audio output without requiring audio files.
    pub fn play_tone(&mut self, freq: f32, duration_secs: f32) {
        use rodio::Source;
        if self.music_sink.is_none() {
            // Create a new sink for music if we don't have one
            if let Ok(sink) = Sink::try_new(&self._stream_handle) {
                sink.set_volume(self.volume);
                self.music_sink = Some(sink);
            } else {
                return;
            }
        }
        let source = rodio::source::SineWave::new(freq)
            .take_duration(std::time::Duration::from_secs_f32(duration_secs))
            .fade_in(std::time::Duration::from_millis(100))
            .amplify(self.volume);
        if let Some(ref sink) = self.music_sink {
            sink.append(source);
        }
    }

    /// Play a simple synthesized ambient audio loop.
    /// Creates a soft, droning ambient sound using layered sine waves.
    pub fn play_music_stub(&mut self) {
        use rodio::Source;
        if self.music_sink.is_none() {
            if let Ok(sink) = Sink::try_new(&self._stream_handle) {
                sink.set_volume(self.volume);
                self.music_sink = Some(sink);
            } else {
                return;
            }
        }

        // Create ambient drone using layered frequencies
        // Base tone (low) + fifth + octave for a pleasant ambient chord
        let base_freq = 110.0;  // A2 - low warm base
        let fifth_freq = 165.0; // E3 - fifth above base
        let octave_freq = 220.0; // A3 - octave above base

        // Mix the frequencies into one continuous ambient source
        // Using take_duration with a long duration, then repeat_infinite
        let ambient = rodio::source::SineWave::new(base_freq)
            .mix(rodio::source::SineWave::new(fifth_freq).amplify(0.5))
            .mix(rodio::source::SineWave::new(octave_freq).amplify(0.3))
            .fade_in(std::time::Duration::from_secs(1))
            .take_duration(std::time::Duration::from_secs(4))
            .repeat_infinite();

        if let Some(ref sink) = self.music_sink {
            sink.append(ambient);
        }
    }

    /// Play per-episode synthesized ambient music.
    /// Different episode themes get different moods using rodio synthesis.
    /// - demo/grass: Upbeat, festive — C4/E4 major arpeggio at 120bpm
    /// - episode2/lava: Tense, dramatic — low G2 with slow LFO modulation at 0.2Hz
    /// - episode3/ice: Ambient, ethereal — high E5 with very slow tremolo
    /// - episode4/night: Mysterious — alternating A3/E3 every 3 seconds
    pub fn play_music_episode(&mut self, episode: &Episode) {
        use rodio::Source;

        // Stop any existing music first
        self.stop_music();

        if let Ok(sink) = Sink::try_new(&self._stream_handle) {
            sink.set_volume(self.volume);
            self.music_sink = Some(sink);
        } else {
            return;
        }

        // Build music based on episode theme
        let music: Box<dyn Source<Item = f32> + Send> = match episode.theme.as_str() {
            // demo/grass: Upbeat, festive — higher frequency (220-330Hz), major key, faster
            "demo" | "grass" => {
                // Upbeat major chord arpeggio: C4-E4-C4-E4 at 120bpm (2 seconds per cycle)
                // C4=262Hz, E4=330Hz
                let beat_duration = std::time::Duration::from_secs_f32(60.0 / 120.0);
                let c4 = rodio::source::SineWave::new(262.0)
                    .take_duration(beat_duration)
                    .fade_in(std::time::Duration::from_millis(20))
                    .amplify(0.4);
                let e4 = rodio::source::SineWave::new(330.0)
                    .take_duration(beat_duration)
                    .fade_in(std::time::Duration::from_millis(20))
                    .amplify(0.4);
                let loop_len = std::time::Duration::from_secs_f32(2.0);
                Box::new(c4.clone().mix(e4.clone()).mix(c4).mix(e4)
                    .take_duration(loop_len)
                    .repeat_infinite())
            }
            // episode2/lava: Tense, dramatic — lower frequency (110-165Hz), minor/diminished, slow pulsing
            "episode2" | "lava" => {
                // Low G2 (98Hz) drone with slow pulsing LFO at 0.2Hz
                // Create a deep, ominous ambient bed
                let base_freq = 98.0; // G2 - deep, ominous
                let pulse_freq = 0.2; // 0.2Hz LFO for slow pulsing effect
                // Layer: G2 + a fifth above (D3=147Hz) + octave above (G3=196Hz)
                let drone = rodio::source::SineWave::new(base_freq)
                    .mix(rodio::source::SineWave::new(147.0).amplify(0.4))
                    .mix(rodio::source::SineWave::new(196.0).amplify(0.3))
                    .fade_in(std::time::Duration::from_secs(2))
                    .take_duration(std::time::Duration::from_secs(5))
                    .repeat_infinite();
                // Amplitude modulation for pulsing effect (0.2Hz = 5 second cycle)
                let pulse = rodio::source::SineWave::new(pulse_freq)
                    .amplify(0.15)
                    .take_duration(std::time::Duration::from_secs(5))
                    .repeat_infinite();
                // Mix the pulse as amplitude modulation
                Box::new(drone.mix(pulse)
                    .take_duration(std::time::Duration::from_secs(5))
                    .repeat_infinite())
            }
            // episode3/ice: Ambient, ethereal — very high (440Hz+), very slow modulation
            "episode3" | "ice" => {
                // High E5 (659Hz) with very slow tremolo and soft harmonics
                // Ethereal: E5 + B5 (fifth) + E6 (octave) with slow 0.1Hz tremolo
                let high_e = rodio::source::SineWave::new(659.0)
                    .mix(rodio::source::SineWave::new(987.5).amplify(0.3)) // B5
                    .mix(rodio::source::SineWave::new(1318.5).amplify(0.2)) // E6
                    .fade_in(std::time::Duration::from_secs(3))
                    .take_duration(std::time::Duration::from_secs(4))
                    .repeat_infinite();
                // Very slow tremolo at 0.1Hz (10 second cycle)
                let tremolo = rodio::source::SineWave::new(0.1)
                    .amplify(0.1)
                    .take_duration(std::time::Duration::from_secs(4))
                    .repeat_infinite();
                Box::new(high_e.mix(tremolo)
                    .take_duration(std::time::Duration::from_secs(4))
                    .repeat_infinite())
            }
            // episode4/night: Mysterious — alternating low tones (165Hz/220Hz), slow oscillation
            "episode4" | "night" => {
                // Alternate between A3 (220Hz) and E3 (165Hz) every 3 seconds
                // A3 + E3 together form a minor interval (minor third)
                let a3 = rodio::source::SineWave::new(220.0)
                    .take_duration(std::time::Duration::from_secs(3))
                    .fade_in(std::time::Duration::from_millis(500))
                    .amplify(0.35);
                let e3 = rodio::source::SineWave::new(165.0)
                    .take_duration(std::time::Duration::from_secs(3))
                    .fade_in(std::time::Duration::from_millis(500))
                    .amplify(0.35);
                // Oscillate between the two tones
                let loop_len = std::time::Duration::from_secs_f32(6.0); // 3s A3 + 3s E3
                Box::new(a3.mix(e3)
                    .take_duration(loop_len)
                    .repeat_infinite())
            }
            // Default fallback: ambient drone
            _ => {
                // Default ambient: A3 drone with soft harmonics
                let a3 = rodio::source::SineWave::new(220.0)
                    .mix(rodio::source::SineWave::new(330.0).amplify(0.3))
                    .mix(rodio::source::SineWave::new(440.0).amplify(0.2))
                    .fade_in(std::time::Duration::from_secs(2))
                    .take_duration(std::time::Duration::from_secs(4))
                    .repeat_infinite();
                Box::new(a3)
            }
        };

        if let Some(ref sink) = self.music_sink {
            sink.append(music);
        }
    }
}

impl Default for AudioManager {
    fn default() -> Self { Self::new() }
}
