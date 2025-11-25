use anyhow::{bail, Context, Result};
use roxmltree::{
    Document,
    Node,
};
use std::str::FromStr;

mod repeat;
mod syllable;
mod verse;
mod word;

use repeat::{
    Repeats,
    RepeatsBuilder,
};
use syllable::{
    Syllable,
};
use verse::Verses;

const CROTCHET: usize = 256;
const MINIM: usize = CROTCHET + CROTCHET;

#[derive(Debug)]
#[derive(PartialEq)]
pub struct Word {
    pub start: usize,
    pub end: usize,
    pub text: String,
}

#[derive(Debug)]
#[derive(PartialEq)]
pub struct Phrase {
    pub start: usize,
    pub end: usize,
    pub words: Vec<Word>,
}

pub struct Part {
    pub phrases: Vec<Phrase>,
}

pub struct Music {
    pub title: Option<String>,
    pub phrases: Vec<Phrase>,
}

pub fn read<'str>(xml: &'str str) -> Result<Option<Music>> {
    let doc = Document::parse_with_options(xml, roxmltree::ParsingOptions {
        allow_dtd: true,
        nodes_limit: u32::MAX,
    }).with_context(|| format!("Reading MUSICXML {}", &xml[..32]))?;
    let root = doc.root_element();
    if !root.has_tag_name("score-partwise") {
        bail!("Expecting root <score-partwise> not {}", root.tag_name().name());
    }

    let Some(part) = has_child_element(root, "part") else {
        bail!("No parts found!");
    };
    let repeats = read_bars(part)?;

    let mut builder = Builder::new(repeats);

    if let Some(work) = has_child_element(root, "work") {
        if let Some(title) = has_child_element(work, "work-title") {
            if let Some(title) = title.text() {
                builder.title(title);
            }
        }
    }

    //let parts = child_element(root, "part-list")?;
    //for part in parts.children() {
    //    if part.has_tag_name("score-part") {
    //        let id = attribute(part, "id")?;
    //        let name = child_element_text(part, "part-name")?;
    //        builder.part_add(id, name);
    //    }
    //}

    for part in root.children() {
        if !part.has_tag_name("part") {
            continue;
        }
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
            read_part_bar(&mut builder, measure)?;
        }
        builder.part_end();
    }

    builder.build()
}

fn read_bars(part: Node) -> Result<Repeats> {
    let mut builder: Option<RepeatsBuilder> = None;

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
        if let Some(builder) = &mut builder {
            builder.next(number)?;
        } else {
            builder = Some(RepeatsBuilder::new(number));
        }
        read_bar(builder.as_mut().unwrap(), measure)?;
    }
    if let Some(builder) = builder {
        builder.build()
    } else {
        bail!("No bars in first part")
    }
}

fn read_bar(builder: &mut RepeatsBuilder, bar: Node) -> Result<()> {
    for node in bar.children() {
        if !node.is_element() {
            continue;
        }
        if node.has_tag_name("attributes") {
            // Key / time signature
        } else if node.has_tag_name("backup") {
            builder.backward(duration_of(node)?);
        } else if node.has_tag_name("barline") {
            read_bar_line(builder, node)?;
        } else if node.has_tag_name("direction") {
            // Dynamics, tempo, ...
        } else if node.has_tag_name("forward") {
            builder.forward(duration_of(node)?);
        } else if node.has_tag_name("harmony") {
            // Chords
        } else if node.has_tag_name("note") {
            if has_child_element(node, "chord").is_some() {
                // skip
            } else if has_child_element(node, "grace").is_some() {
                // skip
            } else {
                builder.forward(duration_of(node)?);
            }
        } else if node.has_tag_name("print") {
            // Layout
        } else {
            bail!("Unexpected <measure><{}>", node.tag_name().name());
        }
    }
    Ok(())
}

fn read_bar_line(builder: &mut RepeatsBuilder, barline: Node) -> Result<()> {
    match barline.attribute("location") {
        Some("left") => {
            // repeat before ending
            if let Some(repeat) = has_child_element(barline, "repeat") {
                match attribute(repeat, "direction")? {
                    "forward" =>
                        (),
                    direction =>
                        bail!("<barline location=`left`><repeat direction=`{direction}`>"),
                }
                builder.repeat_start()?;
            }
            if let Some(ending) = has_child_element(barline, "ending") {
                match attribute(ending, "type")? {
                    "start" =>
                        (),
                    ending =>
                        bail!("<barline location=`left`><ending type=`{ending}`>"),
                }
                let verses: Verses = attribute(ending, "number")?.parse()?;
                builder.ending_start(verses)?;
            }
        }
        Some("right") => {
            // repeat after ending
            if let Some(ending) = has_child_element(barline, "ending") {
                let last = match attribute(ending, "type")? {
                    "stop" =>
                        false,
                    "discontinue" =>
                        true,
                    ending =>
                        bail!("<barline location=`right`><ending type=`{ending}`>"),
                };
                let verses: Verses = attribute(ending, "number")?.parse()?;
                builder.ending_end(verses, last)?;
            }
            if let Some(repeat) = has_child_element(barline, "repeat") {
                match attribute(repeat, "direction")? {
                    "backward" =>
                        (),
                    direction =>
                        bail!("<barline location=`right`><repeat direction=`{direction}`>"),
                }
                builder.repeat_end()?;
            }
        }
        Some(location) =>
            bail!("<barline location=`{location}`>"),
        None =>
            {},
    }
    Ok(())
}

fn read_part_bar<'xml, 'str: 'xml>(
    builder: &mut Builder<'xml>,
    bar: Node<'xml, 'str>,
) -> Result<()> {
    for node in bar.children() {
        if !node.is_element() {
            continue;
        }
        if node.has_tag_name("attributes") {
            // Key / time signature
        } else if node.has_tag_name("backup") {
            builder.backward(duration_of(node)?);
        } else if node.has_tag_name("barline") {
            // repeats already read
        } else if node.has_tag_name("direction") {
            // Dynamics, tempo, ...
        } else if node.has_tag_name("forward") {
            builder.forward(duration_of(node)?);
        } else if node.has_tag_name("harmony") {
            // Chords
        } else if node.has_tag_name("note") {
            read_part_note(builder, node)?;
        } else if node.has_tag_name("print") {
            // Layout
        } else {
            bail!("Unexpected <measure><{}>", node.tag_name().name());
        }
    }
    Ok(())
}

fn read_part_note<'xml, 'str: 'xml>(
    builder: &mut Builder<'xml>,
    note: Node<'xml, 'str>,
) -> Result<()> {
    if has_child_element(note, "chord").is_some() {
        return Ok(());
    }
    if has_child_element(note, "grace").is_some() {
        return Ok(());
    }
    let duration = duration_of(note)?;
    for lyric in note.children() {
        if !lyric.is_element() {
            continue;
        }
        if !lyric.has_tag_name("lyric") {
            continue;
        }
        let voice = child_element_text(note, "voice")?;
        let Ok(voice) = usize::from_str(voice) else {
            bail!("Unexpected <note><voice> `{voice}`")
        };
        if voice == 0 {
            bail!("Unexpected <note><voice> `{voice}`")
        }
        let voice = voice - 1;
        let verse = attribute(lyric, "number")?;
        let verse = if verse.ends_with("verse1") || verse.ends_with("chorus") {
            0
        } else if verse.ends_with("verse2") {
            1
        } else if verse.ends_with("verse3") {
            2
        } else if verse.ends_with("verse4") {
            3
        } else if verse.ends_with("verse5") {
            4
        } else if verse.ends_with("verse6") {
            5
        } else {
            bail!("<lyric number=`{verse}`>");
        };
        let text = child_element_text(lyric, "text")?;
        let kind = child_element_text(lyric, "syllabic")?;
        let kind = syllable::Kind::from_str(kind)?;
        builder.lyric(voice, verse, kind, text, duration);
    }
    builder.forward(duration);
    Ok(())
}

fn has_child_element<'xml, 'str: 'xml>(
    node: Node<'xml, 'str>,
    name: &str,
) -> Option<Node<'xml, 'str>> {
    for child in node.children() {
        if child.has_tag_name(name) {
            return Some(child);
        }
    }

    None
}

fn child_element_text<'xml, 'str: 'xml>(
    node: Node<'xml, 'str>,
    name: &str,
) -> Result<&'xml str> {
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

fn attribute<'xml, 'str: 'xml>(
    node: Node<'xml, 'str>,
    name: &str,
) -> Result<&'xml str> {
    if let Some(attribute) = node.attribute(name) {
        Ok(attribute)
    } else {
        let tag = node.tag_name().name();
        bail!("Expecting attribute `{name}` in <{tag}>")
    }
}

fn duration_of(node: Node) -> Result<usize> {
    let duration = child_element_text(node, "duration")?;
    let Ok(duration) = usize::from_str(duration) else {
        let tag = node.tag_name().name();
        bail!("Unexpected <{tag}><duration> `{duration}`")
    };
    Ok(duration)
}

struct Builder<'xml> {
    title: Option<String>,
    repeats: Repeats,
    next_number: usize,
    bar_tick: usize,
    syllables: syllable::Builder<'xml>,
    parts: Vec<Part>,
}

impl<'dom> Builder<'dom> {
    fn new(repeats: Repeats) -> Self {
        let next_number = repeats.first_bar_number();
        let bar_count = repeats.bar_count();
        Builder {
            title: None,
            repeats,
            next_number,
            bar_tick: 0,
            syllables: syllable::Builder::new(bar_count),
            parts: Vec::new(),
        }
    }

    fn title(&mut self, title: &str) {
        self.title = Some(String::from(title));
    }

    fn part_end(&mut self) {
        self.next_number = self.repeats.first_bar_number();
        self.bar_tick = 0;
        if let Some(part) = self.syllables.part_end(&self.repeats) {
            self.parts.push(part);
        }
    }

    fn bar_start(&mut self, number: usize) -> Result<()> {
        if number != self.next_number {
            bail!("Unexpected bar {number}, expecting {}", self.next_number);
        }
        self.next_number += 1;
        self.bar_tick = 0;
        self.syllables.bar_start();
        Ok(())
    }

    fn backward(&mut self, duration: usize) {
        self.bar_tick -= duration;
    }

    fn forward(&mut self, duration: usize) {
        self.bar_tick += duration;
    }

    fn lyric(
        &mut self,
        voice: usize,
        verse: usize,
        kind: syllable::Kind,
        text: &'dom str,
        duration: usize,
    ) {
        self.syllables.lyric(voice, verse, syllable::Syllable {
            start: self.bar_tick,
            voice,
            end: self.bar_tick + duration,
            kind,
            text,
        });
    }

    fn build(self) -> Result<Option<Music>> {
        let mut parts = self.parts.into_iter();
        if let Some(mut part) = parts.next() {
            let mut from = 0;
            for other in parts {
                for phrase in other.phrases {
                    from = part.merge(phrase, from);
                }
            }
            Ok(Some(Music {
                title: self.title,
                phrases: part.phrases,
            }))
        } else {
            Ok(None)
        }
    }
}

impl Part {
    fn merge(&mut self, other: Phrase, from: usize) -> usize {
        for (index, phrase) in self.phrases.iter().enumerate() {
            if index < from {
                continue;
            }
            if phrase == &other {
                return index;
            }
            if other.start < phrase.start && other.end < phrase.end {
                // debug_phrase("INSERT", &other);
                // debug_phrase("BEFORE", &phrase);
                self.phrases.insert(index, other);
                return index + 1;
            }
        }

        self.phrases.push(other);
        return self.phrases.len();
    }
}

// fn debug_phrase(debug: &str, phrase: &Phrase) {
//     print!("{debug} {}..{} [{}", phrase.start, phrase.end, phrase.words[0].text);
//     for word in &phrase.words[1..] {
//         print!(" {}", word.text);
//     }
//     println!("]");
// }
