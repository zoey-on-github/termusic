pub mod audio_backend;
pub mod config;
pub mod convert;
pub mod decoder;
pub mod dither;
pub mod formatter;
pub mod metadata;
pub mod player;
pub mod playlist;
pub mod tracklist;

pub mod player_service {
    tonic::include_proto!("player_service");
}

#[macro_use]
extern crate log;

pub const SAMPLE_RATE: u32 = 44100;
pub const NUM_CHANNELS: u8 = 2;
pub const SAMPLES_PER_SECOND: u32 = SAMPLE_RATE as u32 * NUM_CHANNELS as u32;
pub const PAGES_PER_MS: f64 = SAMPLE_RATE as f64 / 1000.0;
pub const MS_PER_PAGE: f64 = 1000.0 / SAMPLE_RATE as f64;
