//! ## Utils
//!
//! `Utils` implements utilities functions to work with layouts

// pub mod activity;
/**
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
pub mod components;
pub mod model;

use crate::config::Termusic;
use model::Model;
// Let's define the messages handled by our app. NOTE: it must derive `PartialEq`
use crate::player::GStreamer;
use std::time::Duration;
use tuirealm::application::PollStrategy;
use tuirealm::{Application, AttrValue, Attribute, Update};
// -- internal

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
const FORCED_REDRAW_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Debug, PartialEq)]
pub enum Msg {
    AppClose,
    DigitCounterChanged(isize),
    DigitCounterBlur,
    LibraryTreeExtendDir(String),
    LibraryTreeGoToUpperDir,
    LetterCounterChanged(isize),
    LetterCounterBlur,
    LibraryTreeBlur,
    PlayerTogglePause,
    PlaylistTableBlur,
    PlaylistAdd(String),
    PlaylistDelete(usize),
    PlaylistDeleteAll,
    None,
}

// Let's define the component ids for our application
#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub enum Id {
    GlobalListener,
    DigitCounter,
    LetterCounter,
    Label,
    Library,
    Playlist,
    Progress,
    Lyric,
}

#[derive(Clone, Copy)]
pub enum Status {
    Running,
    Stopped,
    Paused,
}

pub struct UI {
    config: Termusic,
    model: Model,
    pub player: GStreamer,
    status: Option<Status>,
}

impl UI {
    /// Instantiates a new Ui
    pub fn new(config: &Termusic) -> Self {
        let model = Model::new(config);
        // let app = Self::init_application(&model, tick);
        Self {
            config: config.clone(),
            model,
            player: GStreamer::new(),
            status: None,
        }
    }
    /// ### run
    ///
    /// Main loop for Ui thread
    pub fn run(&mut self) {
        self.model.init_terminal();
        assert!(self.model.load_playlist().is_ok());
        self.model.sync_playlist();
        // Main loop
        while !self.model.quit {
            // if let Err(err) = self.app.tick(&mut self.model, PollStrategy::UpTo(3)) {
            //     self.mount_error_popup(format!("Application error: {}", err));
            // }
            // // Poll fetched sources
            // self.poll_fetched_sources();
            // // Run tasks
            // self.run_tasks();
            // // Check whether to force redraw
            // self.check_force_redraw();
            // // View
            // self.model.view(&mut self.app);
            match self.model.app.tick(PollStrategy::Once) {
                Err(err) => {
                    assert!(self
                        .model
                        .app
                        .attr(
                            &Id::Label,
                            Attribute::Text,
                            AttrValue::String(format!("Application error: {}", err)),
                        )
                        .is_ok());
                }
                Ok(messages) if !messages.is_empty() => {
                    // NOTE: redraw if at least one msg has been processed
                    self.model.redraw = true;
                    for msg in messages {
                        let mut msg = Some(msg);
                        while msg.is_some() {
                            msg = self.update(msg);
                            msg = self.model.update(msg);
                        }
                    }
                }

                _ => {}
            }
            // Check whether to force redraw
            self.check_force_redraw();
            self.model.view();
            // // Redraw
            // if self.model.redraw {
            //     self.model.view(&mut self.app);
            //     self.model.redraw = false;
            // }
            match self.status {
                Some(Status::Stopped) => {
                    // if let Some(song) = self.model.
                    if self.model.playlist_items.is_empty() {
                        continue;
                    }
                    self.status = Some(Status::Running);
                    self.next_song();
                }
                None => self.status = Some(Status::Stopped),
                Some(Status::Running | Status::Paused) => {}
            }
            // sleep(Duration::from_millis(2000));
        }
        assert!(self.model.save_playlist().is_ok());
        self.model.finalize_terminal();
    }

    fn check_force_redraw(&mut self) {
        // If source are loading and at least 100ms has elapsed since last redraw...
        if let Some(Status::Running) = self.status {
            if self.model.since_last_redraw() >= FORCED_REDRAW_INTERVAL {
                self.model.force_redraw();
            }
        }
    }

    pub fn next_song(&mut self) {
        if self.model.playlist_items.is_empty() {
            return;
        }
        if let Some(song) = self.model.playlist_items.pop_front() {
            if let Some(file) = song.file() {
                self.player.add_and_play(file);
            }
            self.model.playlist_items.push_back(song.clone());
            // match self.config.loop_mode {
            //     Loop::Playlist => self.playlist_items.push_back(song.clone()),
            //     Loop::Single => self.playlist_items.push_front(song.clone()),
            //     Loop::Queue => {}
            // }
            // self.current_song = Some(song);
            // self.sync_playlist();
            // self.update_photo();
            // self.update_progress_title();
            // self.update_duration();
            // self.update_playing_song();
        }
    }
}
impl Update<Msg> for UI {
    // fn update(&mut self, view: &mut View<Id, Msg, NoUserEvent>, msg: Option<Msg>) -> Option<Msg> {
    fn update(&mut self, msg: Option<Msg>) -> Option<Msg> {
        msg.and_then(|msg| {
            // Set redraw
            self.model.redraw = true;
            // Match message
            match msg {
                Msg::PlayerTogglePause => {
                    self.player.toggle_pause();
                    None
                }
                _ => Some(msg),
            }
        })
    }
}
