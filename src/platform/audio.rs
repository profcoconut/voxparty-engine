use rodio::{Sink, Source, OutputStream, OutputStreamHandle};
use std::collections::HashMap;
use std::fs::File;

/// Simple audio manager using rodio.
pub struct AudioManager {
    _stream: OutputStream,
    _stream_handle: OutputStreamHandle,
    music_sink: Option<Sink>,
    /// Maps sound name → WAV/MP3 file contents
    sfx_data: HashMap<String, Vec<u8>>,
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
        }
    }

    /// Load a sound effect file into memory.
    pub fn load_sfx(&mut self, name: &str, path: &str) {
        if let Ok(data) = std::fs::read(path) {
            self.sfx_data.insert(name.to_string(), data);
        }
    }

    /// Play a sound effect by name.
    /// Falls back to synthesized SFX when no file data is available.
    pub fn play_sfx(&self, name: &str) {
        // Try file-based SFX first
        if let Some(data) = self.sfx_data.get(name) {
            let cursor = std::io::Cursor::new(data.clone());
            if let Ok(decoder) = rodio::Decoder::new(cursor) {
                if let Ok(sink) = Sink::try_new(&self._stream_handle) {
                    sink.append(decoder);
                    return;
                }
            }
        }
        // Fall back to synthesized SFX
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
    fn play_synth_move(&self) {
        use rodio::Source;
        if let Ok(sink) = Sink::try_new(&self._stream_handle) {
            let tone = rodio::source::SineWave::new(880.0)
                .take_duration(std::time::Duration::from_millis(80))
                .fade_in(std::time::Duration::from_millis(5))
                .amplify(0.4);
            sink.append(tone);
        }
    }

    /// Descending dissonant tone for trap/elimination.
    /// Uses two detuned sine waves sweeping down for harsh buzzer effect.
    fn play_synth_trap(&self) {
        use rodio::Source;
        if let Ok(sink) = Sink::try_new(&self._stream_handle) {
            // Descending sine with dissonant second tone
            let descend1 = rodio::source::SineWave::new(400.0)
                .take_duration(std::time::Duration::from_millis(350))
                .fade_in(std::time::Duration::from_millis(10))
                .amplify(0.35);
            let descend2 = rodio::source::SineWave::new(420.0)
                .take_duration(std::time::Duration::from_millis(350))
                .fade_in(std::time::Duration::from_millis(10))
                .amplify(0.25);
            // Low thud underneath
            let thud = rodio::source::SineWave::new(60.0)
                .take_duration(std::time::Duration::from_millis(200))
                .amplify(0.4);
            sink.append(descend1.mix(descend2).mix(thud));
        }
    }

    /// Ascending chime for checkpoint — three rising notes (C5, E5, G5).
    fn play_synth_checkpoint(&self) {
        use rodio::Source;
        if let Ok(sink) = Sink::try_new(&self._stream_handle) {
            let c5 = rodio::source::SineWave::new(523.0)
                .take_duration(std::time::Duration::from_millis(120))
                .fade_in(std::time::Duration::from_millis(10))
                .amplify(0.4);
            let e5 = rodio::source::SineWave::new(659.0)
                .take_duration(std::time::Duration::from_millis(120))
                .fade_in(std::time::Duration::from_millis(10))
                .amplify(0.4);
            let g5 = rodio::source::SineWave::new(784.0)
                .take_duration(std::time::Duration::from_millis(180))
                .fade_in(std::time::Duration::from_millis(10))
                .amplify(0.4);
            sink.append(c5);
            sink.sleep_until_end();
            sink.append(e5);
            sink.sleep_until_end();
            sink.append(g5);
        }
    }

    /// Victory fanfare — short major chord progression: C major → E major → G major → C octave.
    fn play_synth_victory(&self) {
        use rodio::Source;
        if let Ok(sink) = Sink::try_new(&self._stream_handle) {
            // C major chord (C4, E4, G4) — 200ms
            let c_chord = rodio::source::SineWave::new(261.63)
                .mix(rodio::source::SineWave::new(329.63).amplify(0.7))
                .mix(rodio::source::SineWave::new(392.0).amplify(0.6))
                .take_duration(std::time::Duration::from_millis(200))
                .fade_in(std::time::Duration::from_millis(15))
                .amplify(0.45);
            // E major (E4, G#4, B4)
            let e_chord = rodio::source::SineWave::new(329.63)
                .mix(rodio::source::SineWave::new(415.30).amplify(0.7))
                .mix(rodio::source::SineWave::new(493.88).amplify(0.6))
                .take_duration(std::time::Duration::from_millis(200))
                .fade_in(std::time::Duration::from_millis(15))
                .amplify(0.45);
            // G major (G4, B4, D5)
            let g_chord = rodio::source::SineWave::new(392.0)
                .mix(rodio::source::SineWave::new(493.88).amplify(0.7))
                .mix(rodio::source::SineWave::new(587.33).amplify(0.6))
                .take_duration(std::time::Duration::from_millis(200))
                .fade_in(std::time::Duration::from_millis(15))
                .amplify(0.45);
            // C octave (C5) — longer
            let c5 = rodio::source::SineWave::new(523.25)
                .mix(rodio::source::SineWave::new(659.25).amplify(0.5))
                .take_duration(std::time::Duration::from_millis(400))
                .fade_in(std::time::Duration::from_millis(15))
                .amplify(0.45);
            sink.append(c_chord);
            sink.sleep_until_end();
            sink.append(e_chord);
            sink.sleep_until_end();
            sink.append(g_chord);
            sink.sleep_until_end();
            sink.append(c5);
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
                self.music_sink = Some(sink);
            } else {
                return;
            }
        }
        let source = rodio::source::SineWave::new(freq)
            .take_duration(std::time::Duration::from_secs_f32(duration_secs))
            .fade_in(std::time::Duration::from_millis(100));
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

    /// Play per-episode synthesized chiptune music.
    /// Different episode IDs get different melodies using rodio synthesis.
    /// - ep_demo: upbeat 4-note loop (C4-E4-G4-C5 at 120bpm)
    /// - ep_lava_cave: dark minor melody (A4-C5-E5-A4 descending)
    /// - Others: ambient winter theme (soft minor arpeggio)
    pub fn play_music_episode(&mut self, episode_id: &str) {
        use rodio::Source;

        // Stop any existing music first
        self.stop_music();

        if let Ok(sink) = Sink::try_new(&self._stream_handle) {
            self.music_sink = Some(sink);
        } else {
            return;
        }

        // 120bpm = 0.5 seconds per beat
        let beat_duration = std::time::Duration::from_secs_f32(60.0 / 120.0);

        // Build melody based on episode ID
        // Each tuple is (frequency, number_of_beats)
        let music: Box<dyn Source<Item = f32> + Send> = match episode_id {
            "ep_demo" => {
                // Upbeat major chord arpeggio: C4-E4-G4-C5 at 120bpm
                // C4=261.63, E4=329.63, G4=392.0, C5=523.25 Hz
                let c4 = rodio::source::SineWave::new(261.63)
                    .take_duration(beat_duration.mul_f32(1.0))
                    .fade_in(std::time::Duration::from_millis(20))
                    .amplify(0.4);
                let e4 = rodio::source::SineWave::new(329.63)
                    .take_duration(beat_duration.mul_f32(1.0))
                    .fade_in(std::time::Duration::from_millis(20))
                    .amplify(0.4);
                let g4 = rodio::source::SineWave::new(392.0)
                    .take_duration(beat_duration.mul_f32(1.0))
                    .fade_in(std::time::Duration::from_millis(20))
                    .amplify(0.4);
                let c5 = rodio::source::SineWave::new(523.25)
                    .take_duration(beat_duration.mul_f32(1.0))
                    .fade_in(std::time::Duration::from_millis(20))
                    .amplify(0.4);
                // Mix sequentially: c4 then e4 then g4 then c5
                let loop_len = std::time::Duration::from_secs_f32(2.0); // 4 beats * 0.5s
                Box::new(c4.mix(e4).mix(g4).mix(c5)
                    .take_duration(loop_len)
                    .repeat_infinite())
            }
            "ep_lava_cave" => {
                // Dark minor descending: A4-C5-E5-A4 at ~90bpm
                // A4=220, C5=261.63, E5=329.63, A4=220 Hz
                let a4 = rodio::source::SineWave::new(220.0)
                    .take_duration(beat_duration.mul_f32(1.0))
                    .fade_in(std::time::Duration::from_millis(20))
                    .amplify(0.35);
                let c5 = rodio::source::SineWave::new(261.63)
                    .take_duration(beat_duration.mul_f32(0.5))
                    .fade_in(std::time::Duration::from_millis(20))
                    .amplify(0.35);
                let e5 = rodio::source::SineWave::new(329.63)
                    .take_duration(beat_duration.mul_f32(1.0))
                    .fade_in(std::time::Duration::from_millis(20))
                    .amplify(0.35);
                // Ominous descending feel - clone a4 since mix() takes ownership
                let loop_len = std::time::Duration::from_secs_f32(2.5); // ~5 beats * 0.5s
                Box::new(a4.clone().mix(c5).mix(e5).mix(a4)
                    .take_duration(loop_len)
                    .repeat_infinite())
            }
            _ => {
                // Ambient winter theme: soft minor arpeggio A4-C5-E5-A5
                // A4=220, C5=261.63, E5=329.63, A5=440 Hz
                let a4 = rodio::source::SineWave::new(220.0)
                    .take_duration(beat_duration.mul_f32(1.0))
                    .fade_in(std::time::Duration::from_millis(30))
                    .amplify(0.3);
                let c5 = rodio::source::SineWave::new(261.63)
                    .take_duration(beat_duration.mul_f32(1.0))
                    .fade_in(std::time::Duration::from_millis(30))
                    .amplify(0.3);
                let e5 = rodio::source::SineWave::new(329.63)
                    .take_duration(beat_duration.mul_f32(1.0))
                    .fade_in(std::time::Duration::from_millis(30))
                    .amplify(0.3);
                let a5 = rodio::source::SineWave::new(440.0)
                    .take_duration(beat_duration.mul_f32(1.0))
                    .fade_in(std::time::Duration::from_millis(30))
                    .amplify(0.3);
                // Gentle ambient arpeggio - need to clone since mix() takes ownership
                let loop_len = std::time::Duration::from_secs_f32(4.0); // 8 beats * 0.5s
                Box::new(a4.clone().mix(c5.clone()).mix(e5.clone()).mix(a5).mix(e5).mix(c5)
                    .take_duration(loop_len)
                    .repeat_infinite())
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
