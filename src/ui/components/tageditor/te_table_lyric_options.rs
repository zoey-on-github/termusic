//! # Popups
//!
//! Popups components

/**
 * MIT License
 *
 * tuifeed - Copyright (c) 2021 Christian Visintin
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
use crate::songtag::{search, SongTag};
use crate::ui::{Id, Model, Msg, SearchLyricState};

use if_chain::if_chain;
use std::path::Path;
use tui_realm_stdlib::Table;
use tuirealm::command::{Cmd, CmdResult, Direction, Position};
use tuirealm::event::{Key, KeyEvent, KeyModifiers};
use tuirealm::props::{Alignment, BorderType, Borders, Color, TableBuilder, TextSpan};
use tuirealm::{Component, Event, MockComponent, NoUserEvent, State, StateValue};

#[derive(MockComponent)]
pub struct TETableLyricOptions {
    component: Table,
}

impl Default for TETableLyricOptions {
    fn default() -> Self {
        Self {
            component: Table::default()
                .borders(
                    Borders::default()
                        .modifiers(BorderType::Thick)
                        .color(Color::Blue),
                )
                // .foreground(Color::Yellow)
                .background(Color::Black)
                .title("Search Results", Alignment::Left)
                .scroll(true)
                .highlighted_color(Color::LightBlue)
                .highlighted_str("\u{1f680}")
                // .highlighted_str("🚀")
                .rewind(false)
                .step(4)
                .row_height(1)
                .headers(&["Artist", "Title", "Album", "api", "Copyright Info"])
                .column_spacing(3)
                .widths(&[20, 20, 20, 10, 30])
                .table(
                    TableBuilder::default()
                        .add_col(TextSpan::from("0"))
                        .add_col(TextSpan::from(" "))
                        .add_col(TextSpan::from("No Results."))
                        .build(),
                ),
        }
    }
}

impl Component<Msg, NoUserEvent> for TETableLyricOptions {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Msg> {
        let _cmd_result = match ev {
            Event::Keyboard(KeyEvent { code: Key::Tab, .. }) => {
                return Some(Msg::TETableLyricOptionsBlur)
            }
            Event::Keyboard(KeyEvent { code: Key::Esc, .. }) => {
                return Some(Msg::TagEditorBlur(None))
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char('h'),
                modifiers: KeyModifiers::CONTROL,
            }) => return Some(Msg::TEHelpPopupShow),

            Event::Keyboard(KeyEvent {
                code: Key::Down | Key::Char('j'),
                ..
            }) => self.perform(Cmd::Move(Direction::Down)),
            Event::Keyboard(KeyEvent {
                code: Key::Up | Key::Char('k'),
                ..
            }) => self.perform(Cmd::Move(Direction::Up)),
            Event::Keyboard(KeyEvent {
                code: Key::PageDown,
                ..
            }) => self.perform(Cmd::Scroll(Direction::Down)),
            Event::Keyboard(KeyEvent {
                code: Key::PageUp, ..
            }) => self.perform(Cmd::Scroll(Direction::Up)),
            Event::Keyboard(KeyEvent {
                code: Key::Home | Key::Char('g'),
                ..
            }) => self.perform(Cmd::GoTo(Position::Begin)),
            Event::Keyboard(
                KeyEvent { code: Key::End, .. }
                | KeyEvent {
                    code: Key::Char('G'),
                    modifiers: KeyModifiers::SHIFT,
                },
            ) => self.perform(Cmd::GoTo(Position::End)),
            Event::Keyboard(KeyEvent {
                code: Key::Char('s'),
                ..
            }) => return Some(Msg::PlaylistShuffle),
            Event::Keyboard(KeyEvent {
                code: Key::Char('l'),
                ..
            }) => {
                if let State::One(StateValue::Usize(index)) = self.state() {
                    return Some(Msg::PlaylistPlaySelected(index));
                }
                CmdResult::None
            }

            _ => CmdResult::None,
        };
        // match cmd_result {
        // CmdResult::Submit(State::One(StateValue::Usize(_index))) => {
        //     return Some(Msg::PlaylistPlaySelected);
        // }
        //_ =>
        Some(Msg::None)
        // }
    }
}

impl Model {
    pub fn add_songtag_options(&mut self, items: Vec<SongTag>) {
        self.songtag_options = items;
        self.sync_songtag_options();
        assert!(self.app.active(&Id::TETableLyricOptions).is_ok());
    }

    fn sync_songtag_options(&mut self) {
        let mut table: TableBuilder = TableBuilder::default();

        for (idx, record) in self.songtag_options.iter().enumerate() {
            if idx > 0 {
                table.add_row();
            }
            let artist = record.artist().unwrap_or("Nobody");
            let title = record.title().unwrap_or("Unknown Title");
            let album = record.album().unwrap_or("Unknown Album");
            let mut api = "N/A".to_string();
            if let Some(a) = record.service_provider() {
                api = a.to_string();
            }

            let mut url = record.url().unwrap_or_else(|| "No url".to_string());
            if url.starts_with("http") {
                url = "Downloadable".to_string();
            }

            table
                .add_col(TextSpan::new(artist).fg(tuirealm::tui::style::Color::LightYellow))
                .add_col(TextSpan::new(title).bold())
                .add_col(TextSpan::new(album))
                .add_col(TextSpan::new(api))
                .add_col(TextSpan::new(url));
        }
        let table = table.build();
        assert!(self
            .app
            .attr(
                &Id::TETableLyricOptions,
                tuirealm::Attribute::Content,
                tuirealm::AttrValue::Table(table),
            )
            .is_ok());
    }

    pub fn songtag_search(&mut self) {
        let mut search_str = String::new();
        if let Ok(State::One(StateValue::String(artist))) = self.app.state(&Id::TEInputArtist) {
            search_str.push_str(&artist);
        }
        search_str.push(' ');
        if let Ok(State::One(StateValue::String(title))) = self.app.state(&Id::TEInputTitle) {
            search_str.push_str(&title);
        }

        self.mount_error_popup(&search_str);

        if_chain! {
            if search_str.len() < 4;
            if let Some(song) = &self.tageditor_song;
            if let Some(file) = song.file();
            let p: &Path = Path::new(file);
            if let Some(stem) = p.file_stem();

            then {
                search_str = stem.to_string_lossy().to_string();
            }

        }

        search(&search_str, self.sender_songtag.clone());
    }
    pub fn update_lyric_options(&mut self) {
        if let Ok(SearchLyricState::Finish(l)) = self.receiver_songtag.try_recv() {
            self.add_songtag_options(l);
            // self.redraw = true;
        }
    }
}
