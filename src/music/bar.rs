use anyhow::{
    bail,
    Result,
};
use std::ops::Range;

#[derive(Debug)]
struct Bar {
    duration: usize,
}

#[derive(Debug)]
struct Repeat {
    verse: Option<usize>,
    bars: Range<usize>,
}

pub struct Bars {
    first_number: usize,
    bars: Vec<Bar>,
    repeats: Vec<Repeat>,
}

impl Bars {
    pub fn first_number(&self) -> usize {
        self.first_number
    }

    pub fn count(&self) -> usize {
        self.bars.len()
    }

    pub fn iter(&self) -> BarsIter {
        BarsIter {
            bars: &self.bars,
            repeats: self.repeats.iter(),
            verse: None,
            indexes: Range { start: 0, end: 0 },
            tick: 0,
        }
    }
}

pub struct BarsIter<'a> {
    bars: &'a Vec<Bar>,
    repeats: std::slice::Iter<'a, Repeat>,
    verse: Option<usize>,
    indexes: Range<usize>,
    tick: usize,
}

#[derive(Debug)]
pub struct BarIter {
    index: usize,
    verse: Option<usize>,
    tick: usize,
}

impl<'a> std::iter::Iterator for BarsIter<'a> {
    type Item = BarIter;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(index) = self.indexes.next() {
                let item = BarIter {
                    index,
                    verse: self.verse,
                    tick: self.tick,
                };
                let bar = &self.bars[index];
                self.tick += bar.duration;
                return Some(item);
            }
            if let Some(repeat) = self.repeats.next() {
                self.verse = repeat.verse;
                self.indexes = repeat.bars.clone();
            } else {
                return None;
            }
        }
    }
}

pub struct BarsBuilder {
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
    pub fn new(number: usize) -> Self {
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

    pub fn next(&mut self, number: usize) -> Result<()> {
        if number != self.next_number {
            bail!("Unexpected bar {number}, expecting {}", self.next_number);
        }
        self.bars.push(Bar {
            duration: self.max_duration,
        });
        self.next_number += 1;
        self.index += 1;
        self.duration = 0;
        self.max_duration = 0;
        Ok(())
    }

    pub fn forward(&mut self, duration: usize) {
        self.duration += duration;
        if self.max_duration < self.duration {
            self.max_duration = self.duration;
        }
    }

    pub fn backward(&mut self, duration: usize) {
        self.duration -= duration;
        if self.max_duration < self.duration {
            self.max_duration = self.duration;
        }
    }

    pub fn repeat_start(&mut self) {
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

    pub fn repeat_end(&mut self) {
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

    pub fn ending_start(&mut self, verse: Range<usize>) {
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

    pub fn ending_end(&mut self, verse: Range<usize>) {
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

    pub fn build(mut self) -> Result<Bars> {
        self.bars.push(Bar {
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
        Ok(Bars {
            first_number: self.first_number,
            bars: self.bars,
            repeats: self.repeats,
        })
    }
}

