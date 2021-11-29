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
// Locals
use super::{
    TagEditorActivity, COMPONENT_TE_DELETE_LYRIC, COMPONENT_TE_INPUT_ARTIST,
    COMPONENT_TE_INPUT_SONGNAME, COMPONENT_TE_LABEL_HELP, COMPONENT_TE_LABEL_HINT,
    COMPONENT_TE_RADIO_TAG, COMPONENT_TE_SCROLLTABLE_OPTIONS, COMPONENT_TE_SELECT_LYRIC,
    COMPONENT_TE_TEXTAREA_LYRIC, COMPONENT_TE_TEXT_ERROR, COMPONENT_TE_TEXT_HELP,
};
use crate::{
    song::Song,
    ui::{components::counter, draw_area_in},
};
use crate::{
    ui::{Application, Id, Msg},
    VERSION,
};

// Ext
use tuirealm::{
    props::{TableBuilder, TextSpan},
    tui::{
        layout::{Alignment, Constraint, Direction, Layout},
        style::Color,
        widgets::Clear,
    },
};

// tui

impl TagEditorActivity {
    // -- view

    /// ### `init_setup`
    ///
    /// Initialize setup view
    pub fn init_app(tree: &Tree) -> Application<Id, Msg, NoUserEvent> {
        // Setup application
        // NOTE: NoUserEvent is a shorthand to tell tui-realm we're not going to use any custom user event
        // NOTE: the event listener is configured to use the default crossterm input listener and to raise a Tick event each second
        // which we will use to update the clock

        let mut app: Application<Id, Msg, NoUserEvent> = Application::init(
            EventListenerCfg::default()
                .default_input_listener(Duration::from_millis(30))
                .poll_timeout(Duration::from_millis(1000))
                .tick_interval(Duration::from_secs(1)),
        );
        assert!(app
            .mount(Id::Library, Box::new(MusicLibrary::new(tree, None)), vec![])
            .is_ok());
        assert!(app
            .mount(Id::Playlist, Box::new(Playlist::default()), vec![])
            .is_ok());
        assert!(app
            .mount(Id::Progress, Box::new(Progress::default()), vec![])
            .is_ok());
        assert!(app
            .mount(Id::Lyric, Box::new(Lyric::default()), vec![])
            .is_ok());
        assert!(app
            .mount(
                Id::Label,
                Box::new(
                    Label::default()
                        .text(format!("Press <CTRL+H> for help. Version: {}", VERSION,))
                        .alignment(Alignment::Left)
                        .background(Color::Reset)
                        .foreground(Color::Cyan)
                        .modifiers(TextModifiers::BOLD),
                ),
                Vec::default(),
            )
            .is_ok());
        // Mount counters
        assert!(app
            .mount(
                Id::GlobalListener,
                Box::new(GlobalListener::default()),
                Self::subscribe(),
            )
            .is_ok());
        // Active letter counter
        assert!(app.active(&Id::Library).is_ok());
        app
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn init_setup(&mut self) {
        // Init view
        self.view = View::init();
        // Let's mount the component we need
        self.view.mount(
            COMPONENT_TE_LABEL_HINT,
            Box::new(Label::new(
                LabelPropsBuilder::default()
                    .with_foreground(Color::Magenta)
                    .with_text(String::from("Press <ENTER> to search:"))
                    .build(),
            )),
        );

        self.view.mount(
            COMPONENT_TE_LABEL_HELP,
            Box::new(Label::new(
                LabelPropsBuilder::default()
                    .with_foreground(Color::Cyan)
                    .with_text(String::from("Press <CTRL+H> for help."))
                    .build(),
            )),
        );

        self.view.mount(
            COMPONENT_TE_RADIO_TAG,
            Box::new(Radio::new(
                RadioPropsBuilder::default()
                    .with_color(Color::Magenta)
                    .with_borders(
                        Borders::BOTTOM | Borders::TOP,
                        BorderType::Double,
                        Color::Magenta,
                    )
                    .with_inverted_color(Color::Black)
                    .with_value(0)
                    .with_title("Additional operation:", Alignment::Left)
                    .with_options(&["Rename file by Tag"])
                    .build(),
            )),
        );

        self.view.mount(
            COMPONENT_TE_INPUT_ARTIST,
            Box::new(Input::new(
                InputPropsBuilder::default()
                    .with_borders(Borders::ALL, BorderType::Rounded, Color::LightYellow)
                    .with_foreground(Color::Cyan)
                    .with_label(String::from("Search Artist"), Alignment::Left)
                    .build(),
            )),
        );
        self.view.mount(
            COMPONENT_TE_INPUT_SONGNAME,
            Box::new(Input::new(
                InputPropsBuilder::default()
                    .with_borders(Borders::ALL, BorderType::Rounded, Color::LightYellow)
                    .with_foreground(Color::Cyan)
                    .with_label(String::from("Search Song"), Alignment::Left)
                    .build(),
            )),
        );
        // Scrolltable
        self.view.mount(
            COMPONENT_TE_SCROLLTABLE_OPTIONS,
            Box::new(Table::new(
                TablePropsBuilder::default()
                    .with_background(Color::Black)
                    .with_highlighted_str(Some("\u{1f680}"))
                    .with_highlighted_color(Color::LightBlue)
                    .with_max_scroll_step(4)
                    .with_borders(Borders::ALL, BorderType::Rounded, Color::Blue)
                    .with_title("Search Result", Alignment::Left)
                    .scrollable(true)
                    .with_header(&["Artist", "Title", "Album", "api", "Copyright Info"])
                    .with_widths(&[20, 20, 20, 10, 30])
                    .with_table(
                        TableBuilder::default()
                            .add_col(TextSpan::from("0"))
                            .add_col(TextSpan::from(" "))
                            .add_col(TextSpan::from("No Results."))
                            .build(),
                    )
                    .build(),
            )),
        );
        // Lyric Select
        self.view.mount(
            COMPONENT_TE_SELECT_LYRIC,
            Box::new(Select::new(
                SelectPropsBuilder::default()
                    .with_borders(Borders::ALL, BorderType::Rounded, Color::LightRed)
                    .with_background(Color::Black)
                    .with_foreground(Color::LightRed)
                    .with_highlighted_str(Some(">> "))
                    .with_title("Select a lyric", Alignment::Center)
                    .with_options(&["No Lyric".to_string()])
                    .build(),
            )),
        );

        // Lyric Delete
        self.view.mount(
            COMPONENT_TE_DELETE_LYRIC,
            Box::new(counter::Counter::new(
                counter::CounterPropsBuilder::default()
                    .with_borders(Borders::ALL, BorderType::Rounded, Color::LightRed)
                    .with_foreground(Color::Cyan)
                    .with_label(String::from("\nDelete"))
                    .build(),
            )),
        );

        // Lyric Textarea
        self.view.mount(
            COMPONENT_TE_TEXTAREA_LYRIC,
            Box::new(Textarea::new(
                TextareaPropsBuilder::default()
                    .with_foreground(Color::Green)
                    .with_highlighted_str(Some("\u{1f680}"))
                    .with_max_scroll_step(4)
                    .with_borders(Borders::ALL, BorderType::Rounded, Color::LightMagenta)
                    .with_title("Lyrics", Alignment::Left)
                    .with_texts(vec![TextSpan::new("No Lyrics.")
                        .bold()
                        .underlined()
                        .fg(Color::Yellow)])
                    .build(),
            )),
        );

        // We need to initialize the focus
        self.view.active(COMPONENT_TE_INPUT_ARTIST);
    }

    /// View gui
    pub(super) fn view(&mut self) {
        if let Some(mut ctx) = self.context.take() {
            let _drop = ctx.context.draw(|f| {
                // Prepare chunks
                let chunks_main = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(0)
                    .constraints(
                        [
                            Constraint::Length(1),
                            Constraint::Length(3),
                            Constraint::Min(2),
                            Constraint::Length(1),
                        ]
                        .as_ref(),
                    )
                    .split(f.size());

                let chunks_middle1 = Layout::default()
                    .direction(Direction::Horizontal)
                    .margin(0)
                    .constraints(
                        [
                            Constraint::Ratio(1, 4),
                            Constraint::Ratio(2, 4),
                            Constraint::Ratio(1, 4),
                        ]
                        .as_ref(),
                    )
                    .split(chunks_main[1]);
                let chunks_middle2 = Layout::default()
                    .direction(Direction::Horizontal)
                    .margin(0)
                    .constraints([Constraint::Ratio(3, 5), Constraint::Ratio(2, 5)].as_ref())
                    .split(chunks_main[2]);

                let chunks_middle2_right = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(0)
                    .constraints([Constraint::Length(6), Constraint::Min(2)].as_ref())
                    .split(chunks_middle2[1]);

                let chunks_middle2_right_top = Layout::default()
                    .direction(Direction::Horizontal)
                    .margin(0)
                    .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)].as_ref())
                    .split(chunks_middle2_right[0]);

                self.view.render(COMPONENT_TE_LABEL_HINT, f, chunks_main[0]);
                self.view
                    .render(COMPONENT_TE_INPUT_ARTIST, f, chunks_middle1[0]);
                self.view
                    .render(COMPONENT_TE_INPUT_SONGNAME, f, chunks_middle1[1]);
                self.view
                    .render(COMPONENT_TE_RADIO_TAG, f, chunks_middle1[2]);
                self.view
                    .render(COMPONENT_TE_SCROLLTABLE_OPTIONS, f, chunks_middle2[0]);
                self.view.render(COMPONENT_TE_LABEL_HELP, f, chunks_main[3]);

                self.view
                    .render(COMPONENT_TE_SELECT_LYRIC, f, chunks_middle2_right_top[0]);
                self.view
                    .render(COMPONENT_TE_DELETE_LYRIC, f, chunks_middle2_right_top[1]);

                self.view
                    .render(COMPONENT_TE_TEXTAREA_LYRIC, f, chunks_middle2_right[1]);

                if let Some(props) = self.view.get_props(COMPONENT_TE_TEXT_ERROR) {
                    if props.visible {
                        let popup = draw_area_in(f.size(), 50, 10);
                        f.render_widget(Clear, popup);
                        // make popup
                        self.view.render(COMPONENT_TE_TEXT_ERROR, f, popup);
                    }
                }

                if let Some(props) = self.view.get_props(COMPONENT_TE_TEXT_HELP) {
                    if props.visible {
                        // make popup
                        let popup = draw_area_in(f.size(), 50, 70);
                        f.render_widget(Clear, popup);
                        self.view.render(COMPONENT_TE_TEXT_HELP, f, popup);
                    }
                }
            });
            self.context = Some(ctx);
        }
    }

    // -- mount

    // ### mount_error
    //
    // Mount error box
    pub(super) fn mount_error(&mut self, text: &str) {
        // Mount
        self.view.mount(
            COMPONENT_TE_TEXT_ERROR,
            Box::new(Paragraph::new(
                ParagraphPropsBuilder::default()
                    .with_foreground(Color::Red)
                    .bold()
                    .with_borders(Borders::ALL, BorderType::Rounded, Color::Red)
                    .with_title("Error", Alignment::Center)
                    .with_texts(vec![TextSpan::from(text)])
                    .build(),
            )),
        );
        // Give focus to error
        self.view.active(COMPONENT_TE_TEXT_ERROR);
    }

    /// ### `umount_error`
    ///
    /// Umount error message
    pub(super) fn umount_error(&mut self) {
        self.view.umount(COMPONENT_TE_TEXT_ERROR);
    }

    // initialize the value in tageditor based on info from Song
    pub fn init_by_song(&mut self, s: &Song) {
        self.song = Some(s.clone());
        if let Some(artist) = s.artist() {
            if let Some(props) = self.view.get_props(COMPONENT_TE_INPUT_ARTIST) {
                let props = InputPropsBuilder::from(props)
                    .with_value(artist.to_string())
                    .build();
                self.view.update(COMPONENT_TE_INPUT_ARTIST, props);
            }
        }

        if let Some(title) = s.title() {
            if let Some(props) = self.view.get_props(COMPONENT_TE_INPUT_SONGNAME) {
                let props = InputPropsBuilder::from(props)
                    .with_value(title.to_string())
                    .build();
                self.view.update(COMPONENT_TE_INPUT_SONGNAME, props);
            }
        }

        if s.lyric_frames_is_empty() {
            if let Some(props) = self.view.get_props(COMPONENT_TE_SELECT_LYRIC) {
                let props = SelectPropsBuilder::from(props)
                    .with_options(&["Empty"])
                    .build();
                let msg = self.view.update(COMPONENT_TE_SELECT_LYRIC, props);
                self.update(&msg);
            }

            if let Some(props) = self.view.get_props(COMPONENT_TE_DELETE_LYRIC) {
                let props = counter::CounterPropsBuilder::from(props)
                    .with_value(0)
                    .build();
                let msg = self.view.update(COMPONENT_TE_DELETE_LYRIC, props);
                self.update(&msg);
            }

            if let Some(props) = self.view.get_props(COMPONENT_TE_TEXTAREA_LYRIC) {
                let props = TextareaPropsBuilder::from(props)
                    .with_title("Empty Lyrics".to_string(), Alignment::Left)
                    .with_texts(vec![TextSpan::new("No Lyrics.")])
                    .build();
                let msg = self.view.update(COMPONENT_TE_TEXTAREA_LYRIC, props);
                self.update(&msg);
            }

            return;
        }

        let mut vec_lang: Vec<String> = vec![];
        if let Some(lf) = s.lyric_frames() {
            for l in lf {
                vec_lang.push(l.description.clone());
            }
        }
        vec_lang.sort();

        if let Some(props) = self.view.get_props(COMPONENT_TE_SELECT_LYRIC) {
            let props = SelectPropsBuilder::from(props)
                .with_options(&vec_lang)
                .build();
            let msg = self.view.update(COMPONENT_TE_SELECT_LYRIC, props);
            self.update(&msg);
        }

        if let Some(props) = self.view.get_props(COMPONENT_TE_DELETE_LYRIC) {
            let props = counter::CounterPropsBuilder::from(props)
                .with_value(vec_lang.len())
                .build();
            let msg = self.view.update(COMPONENT_TE_DELETE_LYRIC, props);
            self.update(&msg);
        }

        let mut vec_lyric: Vec<TextSpan> = vec![];
        if let Some(f) = s.lyric_selected() {
            for line in f.text.split('\n') {
                vec_lyric.push(TextSpan::from(line));
            }
        }

        if let Some(props) = self.view.get_props(COMPONENT_TE_TEXTAREA_LYRIC) {
            let props = TextareaPropsBuilder::from(props)
                .with_title(
                    format!("{} Lyrics:", vec_lang[s.lyric_selected_index()]),
                    Alignment::Left,
                )
                .with_texts(vec_lyric)
                .build();
            let msg = self.view.update(COMPONENT_TE_TEXTAREA_LYRIC, props);
            self.update(&msg);
        }
    }

    pub(super) fn mount_help(&mut self) {
        self.view.mount(
            COMPONENT_TE_TEXT_HELP,
            Box::new(Table::new(
                TablePropsBuilder::default()
                    .with_borders(Borders::ALL, BorderType::Rounded, Color::Green)
                    .with_title("Help", Alignment::Center)
                    .with_header(&["Key", "Function"])
                    .with_widths(&[25, 75])
                    .with_table(
                        TableBuilder::default()
                            .add_col(TextSpan::new("<ENTER>").bold().fg(Color::Cyan))
                            .add_col(TextSpan::from("Search when focus Artist or Song name."))
                            .add_row()
                            .add_col(TextSpan::new("<ESC> or <Q>").bold().fg(Color::Cyan))
                            .add_col(TextSpan::from("Exit"))
                            .add_row()
                            .add_col(TextSpan::new("<TAB>").bold().fg(Color::Cyan))
                            .add_col(TextSpan::from("Switch focus"))
                            .add_row()
                            .add_col(TextSpan::new("<h,j,k,l,g,G>").bold().fg(Color::Cyan))
                            .add_col(TextSpan::from("Move cursor(vim style)"))
                            .add_row()
                            .add_col(TextSpan::new("<ENTER/l>").bold().fg(Color::Cyan))
                            .add_col(TextSpan::from("Embed selected songtag."))
                            .add_row()
                            .add_col(TextSpan::new("<s>").bold().fg(Color::Cyan))
                            .add_col(TextSpan::from("Download selected song"))
                            .build(),
                    )
                    .build(),
            )),
        );
        // Active help
        self.view.active(COMPONENT_TE_TEXT_HELP);
    }

    /// ### `umount_help`
    ///
    /// Umount help
    pub(super) fn umount_help(&mut self) {
        self.view.umount(COMPONENT_TE_TEXT_HELP);
    }
}
