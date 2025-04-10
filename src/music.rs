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

pub fn read<'str>(xml: &'str str) -> Result<()> {
    let doc = Document::parse_with_options(xml, roxmltree::ParsingOptions {
        allow_dtd: true,
        nodes_limit: u32::MAX,
    })?;
    let root = doc.root_element();
    if !root.has_tag_name("score-partwise") {
        bail!("Expecting root <score-partwise> not {}", root.tag_name().name());
    }

    let Some(part) = has_child_element(root, "part") else {
        bail!("No parts found!");
    };
    let bars = read_bars(part)?;

    let mut builder = Builder::new(bars);

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
            read_part_bar(&mut builder, measure)?;
        }
        builder.part_end();
    }

    Ok(())
}

#[derive(Debug)]
struct Bar {
    number: usize,
    duration: usize,
}

#[derive(Debug)]
struct Repeat {
    verse: Option<usize>,
    bars: Range<usize>,
}

struct Bars {
    first_number: usize,
    bars: Vec<Bar>,
    repeats: Vec<Repeat>,
}

impl Bars {
    fn count(&self) -> usize {
        self.bars.len()
    }

    //fn iter(&self) -> BarsIter {
    //    BarsIter {
    //        bars: self,
    //    }
    //}
}

//struct BarsIter<'a> {
//}

struct BarsBuilder {
    first_number: usize,
    next_number: usize,
    index: usize,
    bars: Vec<Bar>,
    duration: usize,
    max_duration: usize,
    repeat: RepeatBuilder,
    repeats: Vec<Repeat>,
}

#[derive(Debug)]
enum RepeatBuilder {
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
}

impl BarsBuilder {
    fn new(number: usize) -> Self {
        BarsBuilder {
            first_number: number,
            next_number: number + 1,
            index: 0,
            bars: Vec::new(),
            duration: 0,
            max_duration: 0,
            repeat: RepeatBuilder::Normal { start: 0 },
            repeats: Vec::new(),
        }
    }

    fn next(&mut self, number: usize) -> Result<()> {
        if number != self.next_number {
            bail!("Unexpected bar {number}, expecting {}", self.next_number);
        }
        self.bars.push(Bar {
            number: self.first_number + self.index,
            duration: self.max_duration,
        });
        self.next_number += 1;
        self.index += 1;
        self.duration = 0;
        self.max_duration = 0;
        Ok(())
    }

    fn forward(&mut self, duration: usize) {
        self.duration += duration;
        if self.max_duration < self.duration {
            self.max_duration = self.duration;
        }
    }

    fn backward(&mut self, duration: usize) {
        self.duration -= duration;
        if self.max_duration < self.duration {
            self.max_duration = self.duration;
        }
    }

    fn repeat_start(&mut self) {
        match self.repeat {
            RepeatBuilder::Normal { start } => {
                self.repeats.push(Repeat {
                    verse: None,
                    bars: Range {
                        start,
                        end: self.index,
                    },
                });
                self.repeat = RepeatBuilder::Started {
                    start: self.index,
                };
            },

            _ =>
                todo!("REPEAT START {:?}", self.repeat),
        }
    }

    fn repeat_end(&mut self) {
        match &self.repeat {
            RepeatBuilder::Started { start } => {
                // no 1st / 2nd ending bars, so assume repeat twice
                for verse in 0..2 {
                    self.repeats.push(Repeat {
                        verse: Some(verse),
                        bars: Range {
                            start: *start,
                            end: self.index + 1,
                        }
                    });
                }
                self.repeat = RepeatBuilder::Normal {
                    start: self.index + 1,
                };
            }

            RepeatBuilder::Ending { start, ending, verse, closed } => {
                assert_eq!(Some(self.index), *closed);
                for verse in verse.clone() {
                    self.repeats.push(Repeat {
                        verse: Some(verse),
                        bars: Range {
                            start: *start,
                            end: self.index + 1,
                        }
                    });
                }
                if *ending > *start {
                    self.repeats.push(Repeat {
                        verse: Some(verse.end),
                        bars: Range {
                            start: *start,
                            end: *ending,
                        }
                    });
                }
                self.repeat = RepeatBuilder::Ended {
                    start: self.index + 1,
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

    fn ending_start(&mut self, verse: Range<usize>) {
        match &mut self.repeat {
            RepeatBuilder::Started { start } => {
                self.repeat = RepeatBuilder::Ending {
                    start: *start,
                    ending: self.index,
                    verse,
                    closed: None,
                };
            }

            RepeatBuilder::Ended { verse: expect, opened, .. } => {
                assert_eq!(*opened, false);
                assert_eq!(*expect, verse);
                *opened = true;
            }

            _ =>
                todo!("ENDING START {:?}", self.repeat),
        }
    }

    fn ending_end(&mut self, verse: Range<usize>) {
        match &mut self.repeat {
            RepeatBuilder::Ending { closed, verse: opened, .. } => {
                assert_eq!(None, *closed);
                assert_eq!(*opened, verse);
                *closed = Some(self.index);
            }

            RepeatBuilder::Ended { start, verse: expect, opened } => {
                assert_eq!(*opened, true);
                assert_eq!(*expect, verse);
                self.repeat = RepeatBuilder::Normal {
                    start: *start,
                };
            }

            _ =>
                todo!("ENDING STOP {:?}", self.repeat),
        }
    }

    fn build(mut self) -> Result<Bars> {
        self.bars.push(Bar {
            number: self.first_number + self.index,
            duration: self.max_duration,
        });
        match self.repeat {
            RepeatBuilder::Normal { start } => {
                self.repeats.push(Repeat {
                    verse: None,
                    bars: Range {
                        start,
                        end: self.index + 1,
                    },
                });
            }

            _ =>
                todo!("BARS END {:?}", self.repeat),
        }
        println!("BARS: [");
        for bar in &self.bars {
            println!("  {bar:?}");
        }
        println!("]");
        println!("REPEATS: [");
        for repeat in &self.repeats {
            println!("  {repeat:?}");
        }
        println!("]");
        Ok(Bars {
            first_number: self.first_number,
            bars: self.bars,
            repeats: self.repeats,
        })
    }
}

fn read_bars(part: Node) -> Result<Bars> {
    let mut builder: Option<BarsBuilder> = None;

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
            builder = Some(BarsBuilder::new(number));
        }
        read_bar(builder.as_mut().unwrap(), measure)?;
    }
    if let Some(builder) = builder {
        builder.build()
    } else {
        bail!("No bars in first part")
    }
}

fn read_bar(builder: &mut BarsBuilder, bar: Node) -> Result<()> {
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

fn read_bar_line(builder: &mut BarsBuilder, barline: Node) -> Result<()> {
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
                let verse = match attribute(node, "number")? {
                    "1" =>
                        Range { start: 0, end: 1 },
                    "1,2" =>
                        Range { start: 0, end: 2 },
                    "2" =>
                        Range { start: 1, end: 2 },
                    "3" =>
                        Range { start: 2, end: 3 },
                    number =>
                        bail!("<ending number=`{number}`>"),
                };
                if left {
                    builder.ending_start(verse);
                } else {
                    builder.ending_end(verse);
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

fn child_element<'xml, 'str: 'xml>(
    node: Node<'xml, 'str>,
    name: &str,
) -> Result<Node<'xml, 'str>> {
    for child in node.children() {
        if child.has_tag_name(name) {
            return Ok(child);
        }

    }

    let tag = node.tag_name().name();
    bail!("Expecting child <{name}> in <{tag}>")
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

#[derive(Debug)]
#[derive(Eq, PartialEq)]
#[derive(Ord, PartialOrd)]
enum Syllabic {
    Single,
    Begin,
    Middle,
    End,
}

#[derive(Debug)]
#[derive(Eq, PartialEq)]
#[derive(Ord, PartialOrd)]
struct Syllable<'xml> {
    start: usize,
    end: usize,
    is: Syllabic,
    text: &'xml str,
}

#[derive(Debug)]
struct SyllableBar<'xml> {
    verse: [Vec<Syllable<'xml>>; 3],
}

impl<'xml> SyllableBar<'xml> {
    fn new() -> Self {
        SyllableBar {
            verse: [
                Vec::new(),
                Vec::new(),
                Vec::new(),
            ],
        }
    }

    fn sort(&mut self) {
        for verse in &mut self.verse {
            verse.sort();
        }
    }
}

struct Builder<'xml> {
    title: String,
    part_ids: HashMap<String, usize>,
    parts: Vec<Part>,
    bars: Bars,
    part_index: usize,
    next_number: usize,
    bar_index: usize,
    //bar_tick: usize,
    bar_duration: usize,
    //voices: HashMap<(usize, usize), Voice>,
    syllable_exist: bool,
    syllable_bars: Vec<SyllableBar<'xml>>,
    syllable_bar: SyllableBar<'xml>,
}

struct Part {
    name: String,
}

//struct Voice {
//    part: usize,
//    voice: usize,
//    verses: Vec<Verse>,
//}
//
//struct Verse {
//    words: Vec<Word>,
//    word: Option<Word>,
//}
//
//struct Tick {
//    bar: usize,
//    tick: usize,
//}
//
//struct Word {
//    ticks: Range<Tick>,
//    word: String,
//}

impl<'dom> Builder<'dom> {
    fn new(bars: Bars) -> Self {
        let next_number = bars.first_number;
        let bar_count = bars.count();
        Builder {
            title: String::new(),
            part_ids: HashMap::new(),
            parts: Vec::new(),
            bars,
            part_index: 0,
            next_number,
            bar_index: 0,
            //bar_tick: 0,
            bar_duration: 0,
            //voices: HashMap::new(),
            syllable_exist: false,
            syllable_bars: Vec::with_capacity(bar_count),
            syllable_bar: SyllableBar::new(),
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
        let Some(part_index) = self.part_ids.get(id) else {
            bail!("Cannot find part {id}");
        };
        self.part_index = *part_index;
        self.next_number = self.bars.first_number;
        self.bar_index = 0;
        self.bar_duration = 0;
        self.syllable_exist = false;
        Ok(())
    }

    fn part_end(&mut self) {
        let mut bar = std::mem::replace(
            &mut self.syllable_bar,
            SyllableBar::new(),
        );
        bar.sort();
        self.syllable_bars.push(bar);
        let bars = std::mem::replace(
            &mut self.syllable_bars,
            Vec::with_capacity(self.bars.count()),
        );
        if self.syllable_exist {
            println!("LYRICS [");
            for bar in bars {
                println!("  {bar:?}");
            }
            println!("]");
        }
    }

    fn bar_start(&mut self, number: usize) -> Result<()> {
        if number != self.next_number {
            bail!("Unexpected bar {number}, expecting {}", self.next_number);
        }
        let mut bar = std::mem::replace(
            &mut self.syllable_bar,
            SyllableBar::new(),
        );
        bar.sort();
        self.syllable_bars.push(bar);
        self.bar_index = number - self.bars.first_number;
        //self.bar_tick = self.bars.bars[self.bar_index].tick;
        self.bar_duration = 0;
        self.next_number += 1;
        Ok(())
    }

    fn backward(&mut self, duration: usize) {
        self.bar_duration -= duration;
    }

    fn forward(&mut self, duration: usize) {
        self.bar_duration += duration;
    }

    fn lyric(
        &mut self,
        _voice: usize,
        verse: usize,
        is: Syllabic,
        text: &'dom str,
        duration: usize,
    ) {
        let start = self.bar_duration;
        let end = self.bar_duration + duration;
        self.syllable_exist = true;
        self.syllable_bar.verse[verse].push(Syllable {
            start,
            end,
            is,
            text,
        });
    }
}

