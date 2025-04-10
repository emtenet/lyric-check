use anyhow::{
    bail,
    Result,
};

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

    pub fn part_end(&mut self) {
        let mut bar = std::mem::replace(&mut self.bar, Bar::new());
        bar.sort();
        self.bars.push(bar);
        if self.lyrics {
            let bars = std::mem::replace(
                &mut self.bars,
                Vec::with_capacity(self.bar_count),
            );
        } else {
            self.bars.clear();
        }
        self.lyrics = false;
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

