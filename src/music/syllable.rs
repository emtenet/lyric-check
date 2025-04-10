use anyhow::{
    bail,
    Result,
};

use super::{
    Part,
    //Phrase,
    Repeats,
    //Word,
};

#[derive(Copy, Clone)]
#[derive(Debug)]
#[derive(Eq, PartialEq)]
#[derive(Ord, PartialOrd)]
pub enum Kind {
    Single,
    Begin,
    Middle,
    End,
}

impl std::str::FromStr for Kind {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "single" => Ok(Kind::Single),
            "begin" => Ok(Kind::Begin),
            "middle" => Ok(Kind::Middle),
            "end" => Ok(Kind::End),
            _ => bail!("Unknown syllabic kind `{s}`"),
        }
    }
}

#[derive(Debug)]
#[derive(Eq, PartialEq)]
#[derive(Ord, PartialOrd)]
pub struct Syllable<'xml> {
    pub start: usize,
    pub end: usize,
    pub kind: Kind,
    pub text: &'xml str,
}

#[derive(Debug)]
struct Bar<'xml> {
    verse: [Vec<Syllable<'xml>>; 3],
}

impl<'xml> Bar<'xml> {
    fn new() -> Self {
        Bar {
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

    fn iter(&self, verse: Option<usize>, tick: usize) -> Iter {
        if let Some(verse) = verse {
            if verse > 0 && !self.verse[verse].is_empty() {
                return Iter {
                    end: 0,
                    common: Some(self.verse[0].iter()),
                    syllables: self.verse[verse].iter(),
                    tick,
                };
            }
        }
        Iter {
            end: 0,
            common: None,
            syllables: self.verse[0].iter(),
            tick,
        }
    }
}

pub struct Builder<'xml> {
    bar_count: usize,
    bar_first: bool,
    lyrics: bool,
    bars: Vec<Bar<'xml>>,
    bar: Bar<'xml>,
}

impl<'xml> Builder<'xml> {
    pub fn new(bar_count: usize) -> Self {
        Builder {
            bar_count,
            bar_first: true,
            lyrics: false,
            bars: Vec::with_capacity(bar_count),
            bar: Bar::new(),
        }
    }

    pub fn part_end(&mut self, repeats: &Repeats) -> Option<Part> {
        let mut bar = std::mem::replace(&mut self.bar, Bar::new());
        bar.sort();
        self.bars.push(bar);
        self.bar_first = true;
        if self.lyrics {
            self.lyrics = false;
            let bars = std::mem::replace(
                &mut self.bars,
                Vec::with_capacity(self.bar_count),
            );
            let mut builder = super::word::Builder::new();
            for repeat in repeats.bars() {
                for syllable in bars[repeat.index]
                    .iter(repeat.verse, repeat.tick)
                {
                    builder.syllable(syllable);
                }
            }
            Some(builder.build())
        } else {
            self.bars.clear();
            None
        }
    }

    pub fn bar_start(&mut self) {
        if self.bar_first {
            self.bar_first = false;
        } else {
            let mut bar = std::mem::replace(&mut self.bar, Bar::new());
            bar.sort();
            self.bars.push(bar);
        }
    }

    pub fn lyric(
        &mut self,
        _voice: usize,
        verse: usize,
        syllable: Syllable<'xml>,
    ) {
        self.lyrics = true;
        self.bar.verse[verse].push(syllable);
    }
}

struct Iter<'xml> {
    end: usize,
    common: Option<std::slice::Iter<'xml, Syllable<'xml>>>,
    syllables: std::slice::Iter<'xml, Syllable<'xml>>,
    tick: usize,
}

impl<'xml> std::iter::Iterator for Iter<'xml> {
    type Item = Syllable<'xml>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(syllable) = self.syllables.next() {
            self.end = syllable.end;
            Some(Syllable {
                start: syllable.start + self.tick,
                end: syllable.end + self.tick,
                kind: syllable.kind,
                text: syllable.text,
            })
        } else if let Some(mut common) = self.common.take() {
            while let Some(syllable) = common.next() {
                if syllable.start >= self.end {
                    self.syllables = common;
                    return Some(Syllable {
                        start: syllable.start + self.tick,
                        end: syllable.end + self.tick,
                        kind: syllable.kind,
                        text: syllable.text,
                    });
                }
            }
            None
        } else {
            None
        }
    }
}

