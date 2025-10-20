use std::{error, fs, str::Utf8Error};

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
    data: Box<[u8]>,
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
        self.header.subchunk2_size = data_size;
        self.data_size = data_size;
        self.data = data[0..].into();
        data.drain(0..self.data.len());

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
    Ok(s.into()) // Convert &str to Box<str>
}

fn main() -> Result<(), Box<dyn error::Error + 'static>> {
    let mut wav = WAVFile::new();
    let mut data: Vec<u8> = fs::read("./file_example_WAV_5MG.wav")?;

    let _ = wav.parse(&mut data).unwrap();
    println!("{:?}", wav.header);

    Ok(())
}
