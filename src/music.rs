use anyhow::{bail, Result};
use roxmltree::{
    Document,
    Node,
};
use std::collections::HashMap;
use std::ops::Range;
use std::str::FromStr;

//  <part>
//      <measure>
//          <note>
//              <rest/>
//              <lyric number="part14verse1">
//                  <syllabic>begin</syllabic> begin/end/single
//                  <text>Eve</text>
//          <direction>
//              <direction-type>
//                  <words>""

pub fn read<'s>(xml: &'s str) -> Result<()> {
    let mut builder = Builder::new();
    let doc = Document::parse_with_options(xml, roxmltree::ParsingOptions {
        allow_dtd: true,
        nodes_limit: u32::MAX,
    })?;
    let root = doc.root_element();
    if !root.has_tag_name("score-partwise") {
        bail!("Expecting root <score-partwise> not {}", root.tag_name().name());
    }

    let work = child_element(root, "work")?;
    let title = child_element_text(work, "work-title")?;
    builder.title(title);

    let parts = child_element(root, "part-list")?;
    for part in parts.children() {
        if part.has_tag_name("score-part") {
            let id = attribute(part, "id")?;
            let name = child_element_text(part, "part-name")?;
            builder.part_add(id, name);
        }
    }

    for part in root.children() {
        if !part.has_tag_name("part") {
            continue;
        }
        let id = attribute(part, "id")?;
        builder.part_start(id)?;
        for measure in part.children() {
            if !measure.is_element() {
                continue;
            }
            if !measure.has_tag_name("measure") {
                bail!("Unexpected <part><{}>", measure.tag_name().name());
            }
            let number = attribute(measure, "number")?;
            let Ok(number) = usize::from_str(number) else {
                bail!("Unexpected <measure number=`{number}`>")
            };
            builder.bar_start(number)?;
            read_bar(&mut builder, measure)?;
        }
        builder.part_end();
    }

    Ok(())
}

fn read_bar(builder: &mut Builder, bar: Node) -> Result<()> {
    for node in bar.children() {
        if !node.is_element() {
            continue;
        }
        if node.has_tag_name("attributes") {
            // Key / time signature
        } else if node.has_tag_name("backup") {
            let duration = child_element_text(node, "duration")?;
            let Ok(duration) = usize::from_str(duration) else {
                bail!("Unexpected <backup><duration> `{duration}`")
            };
            builder.backup(duration);
        } else if node.has_tag_name("barline") {
            read_barline(builder, node)?;
        } else if node.has_tag_name("direction") {
            // Dynamics, tempo, ...
        } else if node.has_tag_name("forward") {
            let duration = child_element_text(node, "duration")?;
            let Ok(duration) = usize::from_str(duration) else {
                bail!("Unexpected <forward><duration> `{duration}`")
            };
            builder.forward(duration);
        } else if node.has_tag_name("harmony") {
            // Chords
        } else if node.has_tag_name("note") {
            read_note(builder, node)?;
        } else if node.has_tag_name("print") {
            // Layout
        } else {
            bail!("Unexpected <measure><{}>", node.tag_name().name());
        }
    }
    Ok(())
}

fn read_barline(builder: &mut Builder, barline: Node) -> Result<()> {
    if let Some(location) = barline.attribute("location") {
        let left = match location {
            "left" => true,
            "right" => false,
            _ => bail!("<barline location=`{location}`>"),
        };
        for node in barline.children() {
            if node.has_tag_name("ending") {
                match attribute(node, "type")? {
                    "start" =>
                        assert!(left),

                    "stop" | "discontinue" =>
                        assert!(!left),

                    ending =>
                        bail!("<ending type=`{ending}`>"),
                }
                match attribute(node, "number")? {
                    "1" =>
                        builder.repeat_ending(left, Range { start: 1, end: 2 }),
                    "1,2" =>
                        builder.repeat_ending(left, Range { start: 1, end: 3 }),
                    "2" =>
                        builder.repeat_ending(left, Range { start: 2, end: 3 }),
                    "3" =>
                        builder.repeat_ending(left, Range { start: 3, end: 4 }),
                    number =>
                        bail!("<ending number=`{number}`>"),
                }
            } else if node.has_tag_name("repeat") {
                match attribute(node, "direction")? {
                    "forward" => {
                        assert!(left);
                        builder.repeat_start();
                    }

                    "backward" => {
                        assert!(!left);
                        builder.repeat_end();
                    }

                    direction =>
                        bail!("<repeat direction=`{direction}`>"),
                }
            }
        }
    }
    Ok(())
}

fn read_note(builder: &mut Builder, note: Node) -> Result<()> {
    if has_child_element(note, "chord").is_some() {
        return Ok(());
    }
    if has_child_element(note, "grace").is_some() {
        return Ok(());
    }
    let duration = child_element_text(note, "duration")?;
    let Ok(duration) = usize::from_str(duration) else {
        bail!("Unexpected <note><duration> `{duration}`")
    };
    if let Some(lyric) = has_child_element(note, "lyric") {
        let voice = child_element_text(note, "voice")?;
        let Ok(voice) = usize::from_str(voice) else {
            bail!("Unexpected <note><voice> `{voice}`")
        };
        if voice == 0 {
            bail!("Unexpected <note><voice> `{voice}`")
        }
        let voice = voice - 1;
        let verse = attribute(lyric, "number")?;
        let verse = if verse.ends_with("verse1") {
            0
        } else if verse.ends_with("verse2") {
            1
        } else if verse.ends_with("verse3") {
            2
        } else {
            bail!("<lyric number=`{verse}`>");
        };
        let text = child_element_text(lyric, "text")?;
        match child_element_text(lyric, "syllabic")? {
            "single" =>
                builder.lyric(voice, verse, Syllabic::Single, text, duration),

            "begin" =>
                builder.lyric(voice, verse, Syllabic::Begin, text, duration),

            "middle" =>
                builder.lyric(voice, verse, Syllabic::Middle, text, duration),

            "end" =>
                builder.lyric(voice, verse, Syllabic::End, text, duration),

            syllabic =>
                bail!("<lyric><syllabic> `{syllabic}`"),
        }
    } else {
        builder.rest(duration);
    }
    Ok(())
}

fn has_child_element<'a, 'input: 'a>(
    node: Node<'a, 'input>,
    name: &str,
) -> Option<Node<'a, 'input>> {
    for child in node.children() {
        if child.has_tag_name(name) {
            return Some(child);
        }

    }

    None
}

fn child_element<'a, 'input: 'a>(
    node: Node<'a, 'input>,
    name: &str,
) -> Result<Node<'a, 'input>> {
    for child in node.children() {
        if child.has_tag_name(name) {
            return Ok(child);
        }

    }

    let tag = node.tag_name().name();
    bail!("Expecting child <{name}> in <{tag}>")
}

fn child_element_text<'a, 'input: 'a>(
    node: Node<'a, 'input>,
    name: &str,
) -> Result<&'a str> {
    for child in node.children() {
        if child.has_tag_name(name) {
            if let Some(text) = child.text() {
                return Ok(text);
            } else {
                return Ok("");
            }
        }

    }

    let tag = node.tag_name().name();
    bail!("Expecting child <{name}> in <{tag}>")
}

fn attribute<'a, 'input: 'a>(
    node: Node<'a, 'input>,
    name: &str,
) -> Result<&'a str> {
    if let Some(attribute) = node.attribute(name) {
        Ok(attribute)
    } else {
        let tag = node.tag_name().name();
        bail!("Expecting attribute `{name}` in <{tag}>")
    }
}

#[derive(Debug)]
enum Syllabic {
    Single,
    Begin,
    Middle,
    End,
}

struct Builder {
    title: String,
    part_ids: HashMap<String, usize>,
    parts: Vec<Part>,
    part: Option<usize>,
    bar: Option<usize>,
    tick: usize,
    repeat: Repeat,
    verses: Vec<Verse>,
}

struct Part {
    name: String,
}

#[derive(Debug)]
enum Repeat {
    Nil,
    Normal {
        start: usize,
    },
    Started {
        start: usize,
    },
    Ending {
        start: usize,
        ending: usize,
        verse: Range<usize>,
        closed: Option<usize>,
    },
    Ended {
        start: usize,
        verse: Range<usize>,
        opened: bool,
    },
    Complete,
}

#[derive(Debug)]
struct Verse {
    verse: usize,
    bars: Range<usize>,
}

impl Builder {
    fn new() -> Self {
        Builder {
            title: String::new(),
            part_ids: HashMap::new(),
            parts: Vec::new(),
            part: None,
            bar: None,
            tick: 0,
            repeat: Repeat::Nil,
            verses: Vec::new(),
        }
    }

    fn title(&mut self, title: &str) {
        self.title = String::from(title);
    }

    fn part_add(&mut self, id: &str, name: &str) {
        let part = self.parts.len();
        self.part_ids.insert(String::from(id), part);
        self.parts.push(Part {
            name: String::from(name),
        });
    }

    fn part_start(&mut self, id: &str) -> Result<()> {
        self.part = self.part_ids.get(id).copied();
        self.bar = None;
        self.tick = 0;
        if self.part.is_none() {
            bail!("Cannot find part {id}");
        } else {
            Ok(())
        }
    }

    fn part_end(&mut self) {
        let bar = self.bar.unwrap();
        match self.repeat {
            Repeat::Complete => {
            }

            Repeat::Normal { start } => {
                self.verses.push(Verse {
                    verse: 0,
                    bars: Range {
                        start,
                        end: bar + 1,
                    },
                });
                self.repeat = Repeat::Complete;
                println!("VERSES: {:?}", self.verses);
            }

            _ =>
                todo!("PART END {:?}", self.repeat),
        }
    }

    fn bar_start(&mut self, bar: usize) -> Result<()> {
        if let Some(prev) = self.bar {
            if bar != prev + 1 {
                bail!("Unexpected bar {bar}, prev {prev}");
            }
        } else {
            if let Repeat::Nil = self.repeat {
                self.repeat = Repeat::Normal { start: bar };
            }
        }
        self.bar = Some(bar);
        Ok(())
    }

    fn repeat_start(&mut self) {
        let bar = self.bar.unwrap();
        match self.repeat {
            Repeat::Complete => {
            }

            Repeat::Normal { start } => {
                self.verses.push(Verse {
                    verse: 0,
                    bars: Range {
                        start,
                        end: bar,
                    },
                });
                self.repeat = Repeat::Started {
                    start: bar,
                };
            },

            _ =>
                todo!("REPEAT START {:?}", self.repeat),
        }
    }

    fn repeat_end(&mut self) {
        let bar = self.bar.unwrap();
        match &self.repeat {
            Repeat::Complete => {
            }

            Repeat::Started { start } => {
                for verse in 0..2 {
                    self.verses.push(Verse {
                        verse,
                        bars: Range {
                            start: *start,
                            end: bar + 1,
                        }
                    });
                }
                self.repeat = Repeat::Normal {
                    start: bar + 1,
                };
            }

            Repeat::Ending { start, ending, verse, closed } => {
                assert_eq!(Some(bar), *closed);
                for verse in verse.clone() {
                    self.verses.push(Verse {
                        verse,
                        bars: Range {
                            start: *start,
                            end: bar + 1,
                        }
                    });
                }
                if *ending > *start {
                    self.verses.push(Verse {
                        verse: verse.end,
                        bars: Range {
                            start: *start,
                            end: *ending,
                        }
                    });
                }
                self.repeat = Repeat::Ended {
                    start: bar + 1,
                    verse: Range {
                        start: verse.end,
                        end: verse.end + 1,
                    },
                    opened: false,
                };
            }

            _ =>
                todo!("REPEAT END {:?}", self.repeat),
        }
    }

    fn repeat_ending(&mut self, open: bool, verse: Range<usize>) {
        let bar = self.bar.unwrap();
        if open {
            match &mut self.repeat {
                Repeat::Complete => {
                }

                Repeat::Started { start } => {
                    self.repeat = Repeat::Ending {
                        start: *start,
                        ending: bar,
                        verse,
                        closed: None,
                    };
                }

                Repeat::Ended { start, verse: expect, opened } => {
                    assert_eq!(*opened, false);
                    assert_eq!(*expect, verse);
                    *opened = true;
                }

                _ =>
                    todo!("ENDING START {:?}", self.repeat),
            }
        } else {
            match &mut self.repeat {
                Repeat::Complete => {
                }

                Repeat::Ending { closed, verse: opened, .. } => {
                    assert_eq!(None, *closed);
                    assert_eq!(*opened, verse);
                    *closed = Some(bar);
                }

                Repeat::Ended { start, verse: expect, opened } => {
                    assert_eq!(*opened, true);
                    assert_eq!(*expect, verse);
                    self.repeat = Repeat::Normal {
                        start: *start,
                    };
                }

                _ =>
                    todo!("ENDING STOP {:?}", self.repeat),
            }
        }
    }

    fn backup(&mut self, _duration: usize) {
    }

    fn forward(&mut self, _duration: usize) {
    }

    fn rest(&mut self, _duration: usize) {
    }

    fn lyric(
        &mut self,
        _voice: usize,
        _verse: usize,
        _syllabic: Syllabic,
        _text: &str,
        _duration: usize,
    ) {
    }
}

