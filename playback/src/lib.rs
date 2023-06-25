/*
 * MIT License
 *
 * termusic - Copyright (c) 2021 Larry Hao
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

// #![forbid(unsafe_code)]
#![recursion_limit = "2048"]
#![warn(clippy::all, clippy::correctness)]
#![warn(rust_2018_idioms)]
// #![warn(clippy::nursery)]
#![warn(clippy::pedantic)]
#[allow(clippy::pedantic)]
pub mod player {
    tonic::include_proto!("player");
}

mod discord;
#[cfg(all(feature = "gst", not(feature = "mpv")))]
mod gstreamer_backend;
mod mpris;
#[cfg(feature = "mpv")]
mod mpv_backend;
pub mod playlist;
#[cfg(not(any(feature = "mpv", feature = "gst")))]
mod rusty_backend;
use anyhow::Result;
#[cfg(feature = "mpv")]
use mpv_backend::MpvBackend;
pub use playlist::{Playlist, Status};
// use std::sync::RwLock;
// use std::sync::{Arc, Mutex};
use termusiclib::config::{LastPosition, SeekStep, Settings};
// use tokio::sync::Mutex;
// use parking_lot::Mutex;
// use std::sync::Arc;
// use tokio::sync::mpsc::{self, Receiver, Sender};
// #[cfg(not(any(feature = "mpv", feature = "gst")))]
use async_trait::async_trait;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use termusiclib::podcast::db::Database as DBPod;
use termusiclib::sqlite::DataBase;
use termusiclib::track::{MediaType, Track};
use termusiclib::utils::get_app_config_path;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

#[macro_use]
extern crate log;

#[allow(clippy::module_name_repetitions, dead_code)]
pub enum PlayerMsg {
    #[cfg(not(any(feature = "mpv", feature = "gst")))]
    CacheStart(String),
    #[cfg(not(any(feature = "mpv", feature = "gst")))]
    CacheEnd(String),
    #[cfg(not(any(feature = "mpv", feature = "gst")))]
    Duration(u64),
    #[cfg(not(any(feature = "mpv", feature = "gst")))]
    DurationNext(u64),
    Eos,
    AboutToFinish,
    CurrentTrackUpdated,
    Progress(i64, i64),
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum PlayerCmd {
    AboutToFinish,
    CycleLoop,
    #[cfg(not(any(feature = "mpv", feature = "gst")))]
    DurationNext(u64),
    Eos,
    GetProgress,
    PlaySelected,
    SkipPrevious,
    ProcessID,
    ReloadConfig,
    ReloadPlaylist,
    SeekBackward,
    SeekForward,
    SkipNext,
    SpeedDown,
    SpeedUp,
    Tick,
    ToggleGapless,
    TogglePause,
    VolumeDown,
    VolumeUp,
}

/// # Errors
///
///

#[allow(clippy::module_name_repetitions)]
pub struct GeneralPlayer {
    #[cfg(all(feature = "gst", not(feature = "mpv")))]
    pub player: gstreamer_backend::GStreamer,
    #[cfg(feature = "mpv")]
    player: MpvBackend,
    #[cfg(not(any(feature = "mpv", feature = "gst")))]
    player: rusty_backend::Player,
    pub playlist: Playlist,
    pub config: Settings,
    pub need_proceed_to_next: bool,
    pub mpris: mpris::Mpris,
    pub discord: discord::Rpc,
    pub db: DataBase,
    pub db_podcast: DBPod,
    pub cmd_rx: Arc<Mutex<UnboundedReceiver<PlayerCmd>>>,
    pub cmd_tx: Arc<Mutex<UnboundedSender<PlayerCmd>>>,
}

impl GeneralPlayer {
    #[must_use]
    pub fn new(
        config: &Settings,
        cmd_tx: Arc<Mutex<mpsc::UnboundedSender<PlayerCmd>>>,
        cmd_rx: Arc<Mutex<mpsc::UnboundedReceiver<PlayerCmd>>>,
    ) -> Self {
        #[cfg(all(feature = "gst", not(feature = "mpv")))]
        let player = gstreamer_backend::GStreamer::new(config, Arc::clone(&cmd_tx));
        #[cfg(feature = "mpv")]
        let player = MpvBackend::new(config, Arc::clone(&cmd_tx));
        #[cfg(not(any(feature = "mpv", feature = "gst")))]
        let player = rusty_backend::Player::new(config, cmd_tx.clone());
        let playlist = Playlist::new(config).unwrap_or_default();

        let db_path = get_app_config_path().expect("failed to get podcast db path.");

        let db_podcast = DBPod::connect(&db_path).expect("error connecting to podcast db.");
        Self {
            player,
            playlist,
            config: config.clone(),
            need_proceed_to_next: true,
            mpris: mpris::Mpris::default(),
            discord: discord::Rpc::default(),
            db: DataBase::new(config),
            db_podcast,
            cmd_rx,
            cmd_tx,
        }
    }
    pub fn toggle_gapless(&mut self) -> bool {
        self.player.gapless = !self.player.gapless;
        self.config.player_gapless = self.player.gapless;
        self.player.gapless
    }

    pub fn start_play(&mut self) {
        if self.playlist.is_stopped() | self.playlist.is_paused() {
            self.playlist.set_status(Status::Running);
        }

        if self.need_proceed_to_next {
            self.playlist.next();
        } else {
            self.need_proceed_to_next = true;
        }

        if let Some(track) = self.playlist.current_track() {
            let track = track.clone();
            if self.playlist.has_next_track() {
                self.playlist.set_next_track(None);
                info!("gapless next track played");
                #[cfg(not(any(feature = "mpv", feature = "gst")))]
                {
                    {
                        let mut t = self.player.total_duration.lock();
                        *t = self.playlist.next_track_duration();
                    }
                    self.player.message_on_end();

                    self.add_and_play_mpris_discord();
                }
                return;
            }

            let wait = async {
                self.add_and_play(&track).await;
            };
            let rt = tokio::runtime::Runtime::new().expect("failed to create runtime");
            rt.block_on(wait);

            self.add_and_play_mpris_discord();
            self.player_restore_last_position();
            #[cfg(not(any(feature = "mpv", feature = "gst")))]
            {
                self.player.message_on_end();
            }
        }
    }

    fn add_and_play_mpris_discord(&mut self) {
        if let Some(track) = self.playlist.current_track() {
            if self.config.player_use_mpris {
                self.mpris.add_and_play(track);
            }

            if self.config.player_use_discord {
                self.discord.update(track);
            }
        }
    }
    pub fn enqueue_next(&mut self) {
        if self.playlist.next_track().is_some() {
            return;
        }

        let track = match self.playlist.fetch_next_track() {
            Some(t) => t.clone(),
            None => return,
        };

        self.playlist.set_next_track(Some(&track));
        if let Some(file) = track.file() {
            #[cfg(not(any(feature = "mpv", feature = "gst")))]
            self.player.enqueue_next(file);
            // if let Some(d) = self.player.enqueue_next(file) {
            //     self.playlist.set_next_track_duration(d);
            //     // eprintln!("next track queued");
            // }
            #[cfg(all(feature = "gst", not(feature = "mpv")))]
            {
                self.player.enqueue_next(file);
                // eprintln!("next track queued");
                self.playlist.set_next_track(None);
                // self.playlist.handle_current_track();
            }

            #[cfg(feature = "mpv")]
            {
                self.player.enqueue_next(file);
                // eprintln!("next track queued");
            }
        }
    }

    pub fn next(&mut self) {
        if self.playlist.current_track().is_some() {
            info!("skip route 1 which is in most cases.");
            self.playlist.set_next_track(None);
            self.player.skip_one();
        } else {
            info!("skip route 2 cause no current track.");
            self.stop();
            // if let Err(e) = crate::audio_cmd::<()>(PlayerCmd::StartPlay, false) {
            //     debug!("Error in skip route 2: {e}");
            // }
        }
    }
    pub fn previous(&mut self) {
        self.playlist.previous();
        self.need_proceed_to_next = false;
        self.next();
    }
    pub fn toggle_pause(&mut self) {
        match self.playlist.status() {
            Status::Running => {
                self.player.pause();
                if self.config.player_use_mpris {
                    self.mpris.pause();
                }
                if self.config.player_use_discord {
                    self.discord.pause();
                }
                self.playlist.set_status(Status::Paused);
            }
            Status::Stopped => {}
            Status::Paused => {
                self.player.resume();
                if self.config.player_use_mpris {
                    self.mpris.resume();
                }
                if self.config.player_use_discord {
                    let time_pos = self.player.position.lock();
                    self.discord.resume(*time_pos);
                }
                self.playlist.set_status(Status::Running);
            }
        }
    }
    pub fn seek_relative(&mut self, forward: bool) {
        let mut offset = match self.config.player_seek_step {
            SeekStep::Short => -5_i64,
            SeekStep::Long => -30,
            SeekStep::Auto => {
                if let Some(track) = self.playlist.current_track() {
                    if track.duration().as_secs() >= 600 {
                        -30
                    } else {
                        -5
                    }
                } else {
                    -5
                }
            }
        };
        if forward {
            offset = -offset;
        }
        self.player.seek(offset).expect("Error in player seek.");
    }

    #[allow(clippy::cast_sign_loss)]
    pub fn player_save_last_position(&mut self) {
        match self.config.player_remember_last_played_position {
            LastPosition::Yes => {
                if let Some(track) = self.playlist.current_track() {
                    // let time_pos = self.player.position.lock().unwrap();
                    let time_pos = self.player.position.lock();
                    match track.media_type {
                        Some(MediaType::Music) => self
                            .db
                            .set_last_position(track, Duration::from_secs(*time_pos as u64)),
                        Some(MediaType::Podcast) => self
                            .db_podcast
                            .set_last_position(track, Duration::from_secs(*time_pos as u64)),
                        None => {}
                    }
                }
            }
            LastPosition::No => {}
            LastPosition::Auto => {
                if let Some(track) = self.playlist.current_track() {
                    // 10 minutes
                    if track.duration().as_secs() >= 600 {
                        // let time_pos = self.player.position.lock().unwrap();
                        let time_pos = self.player.position.lock();
                        match track.media_type {
                            Some(MediaType::Music) => self
                                .db
                                .set_last_position(track, Duration::from_secs(*time_pos as u64)),
                            Some(MediaType::Podcast) => self
                                .db_podcast
                                .set_last_position(track, Duration::from_secs(*time_pos as u64)),
                            None => {}
                        }
                    }
                }
            }
        }
    }

    // #[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
    pub fn player_restore_last_position(&mut self) {
        let mut restored = false;
        match self.config.player_remember_last_played_position {
            LastPosition::Yes => {
                if let Some(track) = self.playlist.current_track() {
                    match track.media_type {
                        Some(MediaType::Music) => {
                            if let Ok(last_pos) = self.db.get_last_position(track) {
                                self.player.seek_to(last_pos);
                                restored = true;
                            }
                        }

                        Some(MediaType::Podcast) => {
                            if let Ok(last_pos) = self.db_podcast.get_last_position(track) {
                                self.player.seek_to(last_pos);
                                restored = true;
                            }
                        }
                        None => {}
                    }
                }
            }
            LastPosition::No => {}
            LastPosition::Auto => {
                if let Some(track) = self.playlist.current_track() {
                    // 10 minutes
                    if track.duration().as_secs() >= 600 {
                        match track.media_type {
                            Some(MediaType::Music) => {
                                if let Ok(last_pos) = self.db.get_last_position(track) {
                                    self.player.seek_to(last_pos);
                                    restored = true;
                                }
                            }

                            Some(MediaType::Podcast) => {
                                if let Ok(last_pos) = self.db_podcast.get_last_position(track) {
                                    self.player.seek_to(last_pos);
                                    restored = true;
                                }
                            }
                            None => {}
                        }
                    }
                }
            }
        }

        if restored {
            if let Some(track) = self.playlist.current_track() {
                self.db.set_last_position(track, Duration::from_secs(0));
            }
        }
    }
}

#[async_trait]
impl PlayerTrait for GeneralPlayer {
    async fn add_and_play(&mut self, current_track: &Track) {
        self.player.add_and_play(current_track).await;
    }
    fn volume(&self) -> i32 {
        self.player.volume()
    }
    fn volume_up(&mut self) {
        self.player.volume_up();
    }
    fn volume_down(&mut self) {
        self.player.volume_down();
    }
    fn set_volume(&mut self, volume: i32) {
        self.player.set_volume(volume);
    }
    fn pause(&mut self) {
        self.playlist.set_status(Status::Paused);
        self.player.pause();
    }
    fn resume(&mut self) {
        self.playlist.set_status(Status::Running);
        self.player.resume();
    }
    fn is_paused(&self) -> bool {
        self.playlist.is_paused()
    }
    fn seek(&mut self, secs: i64) -> Result<()> {
        self.player.seek(secs)
    }
    fn seek_to(&mut self, last_pos: Duration) {
        self.player.seek_to(last_pos);
    }

    fn set_speed(&mut self, speed: i32) {
        self.player.set_speed(speed);
    }

    fn speed_up(&mut self) {
        self.player.speed_up();
    }

    fn speed_down(&mut self) {
        self.player.speed_down();
    }

    fn speed(&self) -> i32 {
        self.player.speed()
    }

    fn stop(&mut self) {
        self.playlist.set_status(Status::Stopped);
        self.playlist.set_next_track(None);
        self.playlist.clear_current_track();
        self.player.stop();
    }

    fn get_progress(&self) -> Result<(i64, i64)> {
        self.player.get_progress()
    }
}

#[allow(clippy::module_name_repetitions)]
#[async_trait]
pub trait PlayerTrait {
    async fn add_and_play(&mut self, current_track: &Track);
    fn volume(&self) -> i32;
    fn volume_up(&mut self);
    fn volume_down(&mut self);
    fn set_volume(&mut self, volume: i32);
    fn pause(&mut self);
    fn resume(&mut self);
    fn is_paused(&self) -> bool;
    /// # Errors
    ///
    /// Depending on different backend, there could be different errors during seek.
    fn seek(&mut self, secs: i64) -> Result<()>;
    fn seek_to(&mut self, last_pos: Duration);
    /// # Errors
    ///
    /// Depending on different backend, there could be different errors during get progress.
    fn get_progress(&self) -> Result<(i64, i64)>;
    fn set_speed(&mut self, speed: i32);
    fn speed_up(&mut self);
    fn speed_down(&mut self);
    fn speed(&self) -> i32;
    fn stop(&mut self);
}
