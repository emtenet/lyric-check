use anyhow::Result;
use diff::Result as Side;

use super::{
    music,
    script,
    Diff,
    Line,
    Replace,
    Section,
};
use script::Word as Script;

pub fn read(txt: &str, xml: &str) -> Result<Vec<Section>> {
    let music = music::read(xml)?;
    let script = script::read(txt)?;

    let music_words = music_words(&music);
    let script_words = script_words(&script);

    let mut builder = Builder::new();
    for diff in diff::slice(&script_words, &music_words) {
        match diff {
            Side::Left(Word::Script { word: Script::Heading(text), .. }) =>
                builder.heading(text.clone()),

            Side::Left(Word::Script { word: Script::Line(text), .. }) =>
                builder.line(text.clone()),

            Side::Both(
                Word::Script { word: Script::Word(script), .. },
                Word::Music { word: music, .. },
            ) =>
                builder.same(script, music),

            Side::Left(Word::Script { word: Script::Word(script), .. }) =>
                builder.script(script),

            Side::Right(Word::Music { word: music, .. }) =>
                builder.music(music),

            _ =>
                unreachable!("{diff:?}"),
        }
    }

    Ok(builder.build())
}

#[derive(Debug)]
enum Word<'stack> {
    Music {
        word: &'stack music::Word,
        key: String,
    },
    Script {
        word: &'stack script::Word,
        key: String,
    },
}

impl<'stack> Word<'stack> {
    fn key(&self) -> &str {
        match self {
            Word::Music { key, .. } =>
                key.as_str(),

            Word::Script { key, .. } =>
                key.as_str(),
        }
    }
}

impl<'stack> std::cmp::PartialEq for Word<'stack> {
    fn eq(&self, other: &Self) -> bool {
        self.key().eq(other.key())
    }
}

fn music_words(part: &music::Part) -> Vec<Word> {
    let mut words = Vec::new();
    for phrase in &part.phrases {
        for word in &phrase.words {
            words.push(Word::Music {
                word,
                key: key(&word.text),
            });
        }
    }
    words
}

fn script_words(script: &[script::Word]) -> Vec<Word> {
    let mut words = Vec::with_capacity(script.len());
    for word in script {
        match word {
            script::Word::Heading(_) =>
                words.push(Word::Script {
                    word,
                    key: String::from("@HEADING@"),
                }),

            script::Word::Line(_) =>
                words.push(Word::Script {
                    word,
                    key: String::from("@LINE@"),
                }),

            script::Word::Word(text) =>
                words.push(Word::Script {
                    word,
                    key: key(&text),
                }),
        }
    }
    words
}

fn key(text: &str) -> String {
    let mut key = String::with_capacity(text.len());
    for c in text.chars() {
        if c.is_ascii_uppercase() {
            key.push(c.to_ascii_lowercase());
        } else if c.is_ascii_lowercase() {
            key.push(c);
        }
    }
    if key.is_empty() {
        String::from(text)
    } else {
        key
    }
}

struct Builder<'stack> {
    sections: Vec<Section>,
    section: Section,
    line: Line,
    scripts: Vec<&'stack str>,
    musics: Vec<&'stack str>,
}

impl<'stack> Builder<'stack> {
    fn new() -> Self {
        Builder {
            sections: Vec::new(),
            section: Section {
                heading: String::new(),
                lines: Vec::new(),
            },
            line: Line {
                number: String::new(),
                diffs: Vec::new(),
            },
            scripts: Vec::new(),
            musics: Vec::new(),
        }
    }

    fn heading(&mut self, heading: String) {
        self.flush_diff();
        self.flush_line();
        self.flush_section();
        self.section.heading = heading;
    }

    fn line(&mut self, number: String) {
        self.flush_diff();
        self.flush_line();
        self.line.number = number;
    }

    fn same(&mut self, script: &'stack str, music: &'stack music::Word) {
        self.flush_diff();
        diff_word(&mut self.line.diffs, script, &music.text);
    }

    fn script(&mut self, script: &'stack str) {
        self.scripts.push(script);
    }

    fn music(&mut self, music: &'stack music::Word) {
        self.musics.push(&music.text);
    }

    fn flush_section(&mut self) {
        if !self.section.heading.is_empty() || !self.section.lines.is_empty() {
            self.sections.push(std::mem::replace(
                &mut self.section,
                Section {
                    heading: String::new(),
                    lines: Vec::new(),
                },
            ));
        }
    }

    fn flush_line(&mut self) {
        if !self.line.number.is_empty() || !self.line.diffs.is_empty() {
            self.section.lines.push(std::mem::replace(
                &mut self.line,
                Line {
                    number: String::new(),
                    diffs: Vec::new(),
                },
            ));
        }
    }

    fn flush_diff(&mut self) {
        match (&self.scripts[..], &self.musics[..]) {
            ([], []) =>
                (),

            ([script], [music]) => {
                diff_word(&mut self.line.diffs, script, &music);
                self.scripts.clear();
                self.musics.clear();
            }

            ([], musics) => {
                self.line.diffs.push(Diff::Music(
                    musics.join(" ")
                ));
                self.musics.clear();
            }

            (scripts, []) => {
                self.line.diffs.push(Diff::Script(
                    scripts.join(" "),
                ));
                self.scripts.clear();
            }

            (scripts, musics) => {
                self.line.diffs.push(Diff::Replace(Replace {
                    script: scripts.join(" "),
                    music: musics.join(" ")
                }));
                self.scripts.clear();
                self.musics.clear();
            }
        }
    }

    fn build(mut self) -> Vec<Section> {
        self.flush_diff();
        self.flush_line();
        self.flush_section();
        self.sections
    }
}

fn diff_word(diffs: &mut Vec<Diff>, script: &str, music: &str) {
    if script == music {
        diffs.push(Diff::Same(String::from(script)));
        return;
    }

    #[derive(Debug)]
    enum State {
        Empty,
        Same,
        Diff,
    }

    let mut state = State::Empty;
    let mut same = String::new();
    let mut replace = Replace {
        script: String::new(),
        music: String::new(),
    };

    for diff in diff::chars(script, music) {
        match diff {
            Side::Both(c, _) =>
                match state {
                    State::Empty => {
                        state = State::Same;
                        same.push(c);
                    }

                    State::Same =>
                        same.push(c),

                    State::Diff => {
                        let replace = std::mem::replace(
                            &mut replace,
                            Replace {
                                script: String::new(),
                                music: String::new(),
                            },
                        );
                        diffs.push(Diff::Replace(replace));
                        state = State::Same;
                        same.push(c);
                    }
                },

            Side::Left(c) =>
                match state {
                    State::Empty => {
                        replace.script.push(c);
                        state = State::Diff;
                    }

                    State::Same => {
                        let text = std::mem::replace(
                            &mut same, String::new(),
                        );
                        diffs.push(Diff::Same(text));
                        replace.script.push(c);
                        state = State::Diff;
                    }

                    State::Diff =>
                        replace.script.push(c),
                },

            Side::Right(c) =>
                match state {
                    State::Empty => {
                        replace.music.push(c);
                        state = State::Diff;
                    }

                    State::Same => {
                        let text = std::mem::replace(
                            &mut same, String::new(),
                        );
                        diffs.push(Diff::Same(text));
                        replace.music.push(c);
                        state = State::Diff;
                    }

                    State::Diff =>
                        replace.music.push(c),
                },
        }
    }

    match state {
        State::Empty =>
            (),

        State::Same =>
            diffs.push(Diff::Same(same)),

        State::Diff if replace.music.is_empty() =>
            diffs.push(Diff::Script(replace.script)),

        State::Diff if replace.script.is_empty() =>
            diffs.push(Diff::Music(replace.music)),

        State::Diff =>
            diffs.push(Diff::Replace(replace)),
    }
}

