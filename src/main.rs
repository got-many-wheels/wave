use std::sync::{Arc, Mutex};
use std::{error, fs, str::Utf8Error};

use sdl2::audio::{AudioCallback, AudioSpecDesired};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use std::time::Duration;

// http://soundfile.sapp.org/doc/WaveFormat/

#[derive(Default, Debug)]
struct Header {
    // The "RIFF" chunk descriptor
    // The format of concern here is "WAVE", which requires two sub-chunks: "fmt " and "data"
    chunk_id: Box<str>, // 0 - 4
    chunk_size: u32,    // 4 - 8
    format: Box<str>,   // 8 - 12

    // The "fmt " sub-chunk
    // describes the format of the sound information in the data sub-chunk
    subchunk1_id: Box<str>, // 12 - 16
    subchunk1_size: u32,    // 16 - 20
    audio_format: u16,      // 20 - 22
    num_channels: u16,      // 22 - 24
    sample_rate: u32,       // 24 - 28
    byte_rate: u32,         // 28 - 32
    block_align: u16,       // 32 - 34
    bits_per_sample: u16,   // 34 - 36

    // The "data" sub chunk
    subchunk2_id: Box<str>, // 36 - 40
    subchunk2_size: u32,    // 40 - 44
}

#[derive(Default)]
struct WAVFile {
    header: Header,
    // copy of subchunk2_size
    data_size: u32,
    // pointer to data
    data: Box<[i16]>,
}

impl WAVFile {
    fn new() -> Self {
        Self::default()
    }

    fn parse(&mut self, data: &mut Vec<u8>) -> Result<(), Box<dyn error::Error + 'static>> {
        self.header.chunk_id = bytes_to_boxed_str(data).unwrap();
        self.header.chunk_size = little_to_big_u32(data);
        self.header.format = bytes_to_boxed_str(data).unwrap();
        self.header.subchunk1_id = bytes_to_boxed_str(data).unwrap();
        self.header.subchunk1_size = little_to_big_u32(data);
        self.header.audio_format = little_to_big_u16(data);
        self.header.num_channels = little_to_big_u16(data);
        self.header.sample_rate = little_to_big_u32(data);
        self.header.byte_rate = little_to_big_u32(data);
        self.header.block_align = little_to_big_u16(data);
        self.header.bits_per_sample = little_to_big_u16(data);
        self.header.subchunk2_id = bytes_to_boxed_str(data).unwrap();

        let data_size = little_to_big_u32(data);
        self.data_size = data_size;
        self.header.subchunk2_size = data_size;

        if data.len() < data_size as usize {
            return Err("unexpected end of file".into());
        }

        let raw = data.drain(..data_size as usize).collect::<Vec<u8>>();
        // since the buffer we are reading is represented as Vec<u8> we had to convert the audio
        // data to Vec<i16> by combining two elements of idx 0 u8 & 1 u8 to be a single i16
        let mut pcm_data = Vec::with_capacity(raw.len() / 2);
        for chunk in raw.chunks_exact(2) {
            let sample_le = i16::from_le_bytes([chunk[0], chunk[1]]);
            pcm_data.push(sample_le);
        }

        self.data = pcm_data.into_boxed_slice();
        Ok(())
    }
}

fn little_to_big_u32(data: &mut Vec<u8>) -> u32 {
    let value = data[0] as u32
        | ((data[1] as u32) << 8)
        | ((data[2] as u32) << 16)
        | ((data[3] as u32) << 24);
    data.drain(0..4);
    value
}

fn little_to_big_u16(data: &mut Vec<u8>) -> u16 {
    let value = data[0] as u16 | ((data[1] as u16) << 8);
    data.drain(0..2);
    value
}

fn bytes_to_boxed_str(data: &mut Vec<u8>) -> Result<Box<str>, Utf8Error> {
    let bytes = data[0..4].to_vec();
    let s = std::str::from_utf8(&bytes)?;
    data.drain(0..4);
    Ok(s.into())
}

struct AudioPlayer {
    data: Arc<[i16]>,
    position: usize,
    shared_position: Arc<Mutex<usize>>,
}

impl AudioCallback for AudioPlayer {
    type Channel = i16;

    fn callback(&mut self, out: &mut [i16]) {
        for sample in out.iter_mut() {
            *sample = if self.position < self.data.len() {
                self.data[self.position]
            } else {
                0
            };
            self.position += 1;
        }

        // Update shared position for rendering
        *self.shared_position.lock().unwrap() = self.position;
    }
}

fn main() -> Result<(), Box<dyn error::Error + 'static>> {
    let mut wav = WAVFile::new();
    let mut data = fs::read("file_example_WAV_5MG.wav")?;
    let _ = wav.parse(&mut data).unwrap();

    let sdl_context = sdl2::init().unwrap();

    let shared_position = Arc::new(Mutex::new(0));
    let player = AudioPlayer {
        data: wav.data.clone().into(),
        position: 0,
        shared_position: shared_position.clone(),
    };
    let desired_spec = AudioSpecDesired {
        freq: Some(wav.header.sample_rate as i32),
        channels: Some(wav.header.num_channels as u8),
        samples: Some(wav.header.bits_per_sample),
    };

    let audio_subsystem = sdl_context.audio().unwrap();

    // use callback since we want to syncronize the samples position in the audio buffer
    let device = audio_subsystem.open_playback(None, &desired_spec, |_spec| player)?;
    device.resume();

    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("wave", 800, 600)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();

    canvas.set_draw_color(Color::RGB(0, 255, 255));
    canvas.clear();
    canvas.present();
    let mut event_pump = sdl_context.event_pump().unwrap();
    device.resume();

    'running: loop {
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();
        let played_samples = *shared_position.lock().unwrap();
        draw_waveform(&mut canvas, &wav, played_samples);

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                _ => {}
            }
        }

        canvas.present();
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }
    Ok(())
}

fn draw_waveform(
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    wav: &WAVFile,
    played_samples: usize,
) {
    let (width, height) = canvas.output_size().unwrap();
    let samples_to_display = 4096;

    let start = played_samples;
    let end = (start + samples_to_display).min(wav.data.len());

    if start >= wav.data.len() {
        return;
    }

    let chunk = &wav.data[start..end];

    canvas.set_draw_color(Color::RGB(0, 255, 0));

    let center_y = height as i32 / 2;

    for i in 0..chunk.len().saturating_sub(1) {
        let x1 = (i as f32 / samples_to_display as f32 * width as f32) as i32;
        let x2 = ((i + 1) as f32 / samples_to_display as f32 * width as f32) as i32;

        let y1 = center_y - (chunk[i] as i32 * height as i32 / 2 / 32768);
        let y2 = center_y - (chunk[i + 1] as i32 * height as i32 / 2 / 32768);

        canvas.draw_line((x1, y1), (x2, y2)).ok();
    }
}
