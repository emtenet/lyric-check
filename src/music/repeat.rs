use anyhow::{
    bail,
    Result,
};
use std::collections::BTreeMap;
use std::ops::Range;
use super::Verses;

#[derive(Debug)]
pub struct Bar {
    pub index: usize,
    pub verse: Option<usize>,
    pub tick: usize,
}

#[derive(Debug)]
#[derive(PartialEq)]
struct Repeat {
    verse: Option<usize>,
    bars: Range<usize>,
}

pub struct Repeats {
    first_bar_number: usize,
    durations: Vec<usize>,
    repeats: Vec<Repeat>,
}

impl Repeats {
    pub fn first_bar_number(&self) -> usize {
        self.first_bar_number
    }

    pub fn bar_count(&self) -> usize {
        self.durations.len()
    }

    pub fn bars(&self) -> Bars<'_> {
        Bars {
            durations: &self.durations,
            repeats: self.repeats.iter(),
            verse: None,
            indexes: Range { start: 0, end: 0 },
            tick: 0,
        }
    }
}

pub struct Bars<'a> {
    durations: &'a Vec<usize>,
    repeats: std::slice::Iter<'a, Repeat>,
    verse: Option<usize>,
    indexes: Range<usize>,
    tick: usize,
}

impl<'a> std::iter::Iterator for Bars<'a> {
    type Item = Bar;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(index) = self.indexes.next() {
                let item = Bar {
                    index,
                    verse: self.verse,
                    tick: self.tick,
                };
                let duration = &self.durations[index];
                self.tick += duration;
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

pub struct RepeatsBuilder {
    bar_to_number: usize,
    bar: usize,
    durations: Vec<usize>,
    duration: usize,
    max_duration: usize,
    state: RepeatBuilder,
    common: Range<usize>,
    endings: BTreeMap<usize, Repeat>,
    repeats: Vec<Repeat>,
}

#[derive(Debug)]
enum RepeatBuilder {
    Normal {
        bar: usize,
    },
    RepeatStart {
        bar: usize,
    },
    EndingStart {
        bar: usize,
        verses: Verses,
    },
    EndingStop {
        bar: usize,
        verses: Verses,
    },
    RepeatStop {
        bar: usize,
    },
}

impl RepeatsBuilder {
    pub fn new(number: usize) -> Self {
        RepeatsBuilder {
            bar_to_number: number,
            bar: 0,
            durations: Vec::new(),
            duration: 0,
            max_duration: 0,
            state: RepeatBuilder::Normal {
                bar: 0,
            },
            common: Range {
                start: 0,
                end: 0,
            },
            endings: BTreeMap::new(),
            repeats: Vec::new(),
        }
    }

    pub fn next(&mut self, number: usize) -> Result<()> {
        let expect_number = self.bar + 1 + self.bar_to_number;
        if number != expect_number {
            bail!("Unexpected bar {number}, expecting {expect_number}");
        }
        self.durations.push(self.max_duration);
        self.bar += 1;
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

    pub fn repeat_start(&mut self) -> Result<()> {
        match &self.state {
            RepeatBuilder::Normal { bar } => {
                if self.bar > *bar {
                    self.repeats.push(Repeat {
                        verse: None,
                        bars: Range {
                            start: *bar,
                            end: self.bar,
                        },
                    });
                }
                self.state = RepeatBuilder::RepeatStart {
                    bar: self.bar,
                };
            },

            RepeatBuilder::RepeatStop { bar } => {
                let after = Range {
                    start: *bar,
                    end: self.bar,
                };
                self.repeat_open(after)?;
                self.state = RepeatBuilder::RepeatStart {
                    bar: self.bar,
                };
            }

            _ =>
                bail!("BAR {} REPEAT START {:?}", self.bar, self.state),
        }
        Ok(())
    }

    pub fn ending_start(&mut self, verses: Verses) -> Result<()> {
        match &self.state {
            RepeatBuilder::Normal { bar } if *bar > 0 =>
                bail!("Ending at bar {} with no start of repeat",
                    self.bar + 1 + self.bar_to_number,
                ),

            RepeatBuilder::Normal { bar } |
            RepeatBuilder::RepeatStart { bar } => {
                self.common = Range {
                    start: *bar,
                    end: self.bar,
                };
                self.endings.clear();
                self.state = RepeatBuilder::EndingStart {
                    bar: self.bar,
                    verses,
                };
            }

            RepeatBuilder::RepeatStop { bar } => {
                if self.bar != *bar {
                    bail!("Alternative ending must start straight after end of repeat at bar {}",
                        self.bar + self.bar_to_number,
                    )
                }
                if self.endings.is_empty() && verses == Verses::TWO {
                    self.endings.insert(0, Repeat {
                        verse: None,
                        bars: Range {
                            start: *bar,
                            end: *bar,
                        },
                    });
                }
                self.state = RepeatBuilder::EndingStart {
                    bar: *bar,
                    verses,
                };
            }

            _ =>
                bail!("BAR {} ENDING START {:?} VERSE {verses:?}", self.bar, self.state),
        }
        Ok(())
    }

    pub fn ending_end(&mut self, verses: Verses, last: bool) -> Result<()> {
        match &self.state {
            RepeatBuilder::EndingStart { bar, verses: expect } => {
                assert_eq!(verses, *expect);
                if let Some(verse) = verses.to_single() {
                    if let Some(dup) = self.endings.get(&verse) {
                        bail!("Ending {} at bar {} is duplicated as bar {}",
                            verse + 1,
                            dup.bars.start + self.bar_to_number,
                            bar + self.bar_to_number,
                        )
                    }
                    self.endings.insert(verse, Repeat {
                        verse: None,
                        bars: Range {
                            start: *bar,
                            end: self.bar + 1,
                        },
                    });
                } else {
                    for (ending, verse) in verses.clone().enumerate() {
                        if let Some(dup) = self.endings.get(&verse) {
                            bail!("Ending {} at bar {} is duplicated as bar {}",
                                verse + 1,
                                dup.bars.start + self.bar_to_number,
                                bar + self.bar_to_number,
                            )
                        }
                        self.endings.insert(verse, Repeat {
                            verse: Some(ending),
                            bars: Range {
                                start: *bar,
                                end: self.bar + 1,
                            },
                        });
                    }
                }
                if last {
                    if let Some(last) = verses.to_single() {
                        self.repeat_closed(0..last + 1)?;
                        self.state = RepeatBuilder::Normal {
                            bar: self.bar + 1,
                        };
                    } else {
                        bail!("Final ending at bar {} must be singular",
                            bar + self.bar_to_number,
                        );
                    }
                } else {
                    self.state = RepeatBuilder::EndingStop {
                        bar: self.bar + 1,
                        verses,
                    };
                }
            }

            _ =>
                bail!("BAR {} ENDING STOP {:?} VERSE {verses:?} LAST {last}", self.bar, self.state),
        }
        Ok(())
    }

    pub fn repeat_end(&mut self) -> Result<()> {
        match self.state {
            RepeatBuilder::Normal { bar } if bar > 0 =>
                bail!("End of repeat at bar {} with no start of repeat",
                    self.bar + self.bar_to_number,
                ),

            RepeatBuilder::Normal { bar } |
            RepeatBuilder::RepeatStart { bar } => {
                self.common = Range {
                    start: bar,
                    end: self.bar + 1,
                };
                self.endings.clear();
                self.state = RepeatBuilder::RepeatStop {
                    bar: self.bar + 1,
                };
            }

            RepeatBuilder::EndingStop { bar, .. } => {
                if bar != self.bar + 1 {
                    bail!("Gap between ending at bar {} and repeat at bar {}",
                        bar + self.bar_to_number,
                        self.bar + self.bar_to_number,
                    )
                }
                self.state = RepeatBuilder::RepeatStop {
                    bar,
                };
            }

            _ =>
                bail!("BAR {} REPEAT END {:?}", self.bar, self.state),
        }
        Ok(())
    }

    fn repeat_open(&mut self, after: Range<usize>) -> Result<()> {
        if let Some((last, _)) = self.endings.last_key_value() {
            self.repeat_closed(0..last + 1)?;
        } else {
            // assume repeat twice
            assert!(!self.common.is_empty());
            self.repeats.push(Repeat {
                verse: Some(0),
                bars: self.common.clone(),
            });
            self.repeats.push(Repeat {
                verse: Some(1),
                bars: self.common.clone(),
            });
        }
        if !after.is_empty() {
            self.repeats.push(Repeat {
                verse: None,
                bars: after,
            });
        }
        Ok(())
    }

    fn repeat_closed(&mut self, verses: Range<usize>) -> Result<()> {
        for verse in verses {
            if !self.common.is_empty() {
                self.repeats.push(Repeat {
                    verse: Some(verse),
                    bars: self.common.clone(),
                });
            }
            if let Some(ending) = self.endings.remove(&verse) {
                self.repeats.push(ending);
            } else {
                bail!("Repeat at bar {} missing ending {}",
                    self.bar + self.bar_to_number,
                    verse + 1,
                )
            }
        }
        Ok(())
    }

    pub fn build(mut self) -> Result<Repeats> {
        self.durations.push(self.max_duration);
        match &self.state {
            RepeatBuilder::Normal { bar } => {
                if self.bar >= *bar {
                    self.repeats.push(Repeat {
                        verse: None,
                        bars: Range {
                            start: *bar,
                            end: self.bar + 1,
                        },
                    });
                }
            }

            RepeatBuilder::RepeatStart { bar } if *bar > 0 =>
                bail!("Start of repeat at bar {} with no end of repeat",
                    *bar + self.bar_to_number,
                ),

            RepeatBuilder::EndingStart { bar, verses } => {
                if let Some(verse) = verses.to_single() {
                    if let Some(dup) = self.endings.get(&verse) {
                        bail!("Ending {} at bar {} is duplicated as bar {}",
                            verse + 1,
                            dup.bars.start + self.bar_to_number,
                            bar + self.bar_to_number,
                        )
                    }
                    self.endings.insert(verse, Repeat {
                        verse: None,
                        bars: Range {
                            start: *bar,
                            end: self.bar + 1,
                        },
                    });
                    self.repeat_closed(0..verse + 1)?;
                } else {
                    bail!("Final ending at bar {} must be singular",
                        bar + self.bar_to_number,
                    );
                }
            }

            RepeatBuilder::EndingStop { bar, verses } => {
                let bar = *bar;
                if let Some(last) = verses.to_single() {
                    self.repeat_closed(0..last + 1)?;
                    if self.bar >= bar {
                        self.repeats.push(Repeat {
                            verse: None,
                            bars: Range {
                                start: bar,
                                end: self.bar + 1,
                            },
                        });
                    }
                } else {
                    bail!("Final ending at bar {} must be singular",
                        bar + self.bar_to_number,
                    );
                }
            }

            RepeatBuilder::RepeatStop { bar } => {
                let after = Range {
                    start: *bar,
                    end: self.bar + 1,
                };
                self.repeat_open(after)?;
            }

            _ =>
                bail!("BARS END {:?}", self.state),
        }
        Ok(Repeats {
            first_bar_number: self.bar_to_number,
            durations: self.durations,
            repeats: self.repeats,
        })
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use super::*;

    fn new(bars: usize) -> Result<RepeatsBuilder> {
        assert!(bars > 0);
        let mut repeats = RepeatsBuilder::new(1);
        repeats.forward(1024);
        for bar in 1..bars {
            repeats.next(bar + 1)?;
            repeats.forward(1024);
        }
        Ok(repeats)
    }

    fn new_repeat(bars: usize) -> Result<RepeatsBuilder> {
        assert!(bars > 0);
        let mut repeats = RepeatsBuilder::new(1);
        repeats.repeat_start()?;
        repeats.forward(1024);
        for bar in 1..bars {
            repeats.next(bar + 1)?;
            repeats.forward(1024);
        }
        repeats.repeat_end()?;
        Ok(repeats)
    }

    fn new_repeat_start(bars: usize) -> Result<RepeatsBuilder> {
        assert!(bars > 0);
        let mut repeats = RepeatsBuilder::new(1);
        repeats.repeat_start()?;
        repeats.forward(1024);
        for bar in 1..bars {
            repeats.next(bar + 1)?;
            repeats.forward(1024);
        }
        Ok(repeats)
    }

    fn normal(repeats: &mut RepeatsBuilder, bars: Range<usize>) -> Result<()> {
        for bar in bars {
            repeats.next(bar + 1)?;
            repeats.forward(1024);
        }
        Ok(())
    }

    fn repeat(repeats: &mut RepeatsBuilder, mut bars: Range<usize>) -> Result<()> {
        repeats.next(bars.start + 1)?;
        repeats.repeat_start()?;
        repeats.forward(1024);
        bars.start += 1;
        for bar in bars {
            repeats.next(bar + 1)?;
            repeats.forward(1024);
        }
        repeats.repeat_end()?;
        Ok(())
    }

    fn repeat_start(repeats: &mut RepeatsBuilder, mut bars: Range<usize>) -> Result<()> {
        repeats.next(bars.start + 1)?;
        repeats.repeat_start()?;
        repeats.forward(1024);
        bars.start += 1;
        for bar in bars {
            repeats.next(bar + 1)?;
            repeats.forward(1024);
        }
        Ok(())
    }

    fn ending(repeats: &mut RepeatsBuilder, verses: Verses, mut bars: Range<usize>) -> Result<()> {
        repeats.next(bars.start + 1)?;
        repeats.ending_start(verses.clone())?;
        repeats.forward(1024);
        bars.start += 1;
        for bar in bars {
            repeats.next(bar + 1)?;
            repeats.forward(1024);
        }
        repeats.ending_end(verses, false)?;
        repeats.repeat_end()?;
        Ok(())
    }

    fn ending_last(repeats: &mut RepeatsBuilder, verses: Verses, mut bars: Range<usize>) -> Result<()> {
        repeats.next(bars.start + 1)?;
        repeats.ending_start(verses.clone())?;
        repeats.forward(1024);
        bars.start += 1;
        for bar in bars {
            repeats.next(bar + 1)?;
            repeats.forward(1024);
        }
        repeats.ending_end(verses, true)?;
        Ok(())
    }

    #[test]
    fn test_no_repeats() -> Result<()> {
        let repeats = new(4)?;
        // build
        let repeats = repeats.build()?;
        assert_eq!(repeats.repeats, vec![
            Repeat { verse: None, bars: 0..4 },
        ]);
        Ok(())
    }

    #[test]
    fn test_ending_not_in_repeat() -> Result<()> {
        let mut repeats = RepeatsBuilder::new(1);
        // MISSING repeats.repeat_start()?;
        repeats.forward(1024);
        // start of ending
        repeats.next(2)?;
        assert!(repeats.ending_start(Verses::ONE).is_err());
        Ok(())
    }

    #[test]
    fn test_repeat_not_started() -> Result<()> {
        let mut repeats = RepeatsBuilder::new(1);
        // MISSING repeats.repeat_start()?;
        repeats.forward(1024);
        // end of repeat
        repeats.next(2)?;
        repeats.forward(1024);
        assert!(repeats.repeat_end().is_err());
        Ok(())
    }

    #[test]
    fn test_repeat() -> Result<()> {
        let repeats = new_repeat(4)?;
        // build
        let repeats = repeats.build()?;
        assert_eq!(repeats.repeats, vec![
            Repeat { verse: Some(0), bars: 0..4 },
            Repeat { verse: Some(1), bars: 0..4 },
        ]);
        Ok(())
    }

    #[test]
    fn test_repeat_repeat() -> Result<()> {
        let mut repeats = new_repeat(4)?;
        normal(&mut repeats, 4..8)?;
        repeat(&mut repeats, 8..12)?;
        // build
        let repeats = repeats.build()?;
        assert_eq!(repeats.repeats, vec![
            Repeat { verse: Some(0), bars: 0..4 },
            Repeat { verse: Some(1), bars: 0..4 },
            Repeat { verse: None, bars: 4..8 },
            Repeat { verse: Some(0), bars: 8..12 },
            Repeat { verse: Some(1), bars: 8..12 },
        ]);
        Ok(())
    }

    #[test]
    fn test_intro_repeat() -> Result<()> {
        let mut repeats = new(4)?;
        repeat(&mut repeats, 4..8)?;
        // build
        let repeats = repeats.build()?;
        assert_eq!(repeats.repeats, vec![
            Repeat { verse: None, bars: 0..4 },
            Repeat { verse: Some(0), bars: 4..8 },
            Repeat { verse: Some(1), bars: 4..8 },
        ]);
        Ok(())
    }

    #[test]
    fn test_intro_repeat_outro() -> Result<()> {
        let mut repeats = new(4)?;
        repeat(&mut repeats, 4..8)?;
        normal(&mut repeats, 8..12)?;
        // build
        let repeats = repeats.build()?;
        assert_eq!(repeats.repeats, vec![
            Repeat { verse: None, bars: 0..4 },
            Repeat { verse: Some(0), bars: 4..8 },
            Repeat { verse: Some(1), bars: 4..8 },
            Repeat { verse: None, bars: 8..12 },
        ]);
        Ok(())
    }

    #[test]
    fn test_repeat_outro() -> Result<()> {
        let mut repeats = new_repeat(4)?;
        normal(&mut repeats, 4..8)?;
        // build
        let repeats = repeats.build()?;
        assert_eq!(repeats.repeats, vec![
            Repeat { verse: Some(0), bars: 0..4 },
            Repeat { verse: Some(1), bars: 0..4 },
            Repeat { verse: None, bars: 4..8 },
        ]);
        Ok(())
    }

    #[test]
    fn test_first_second() -> Result<()> {
        let mut repeats = new_repeat_start(4)?;
        ending(&mut repeats, Verses::ONE, 4..8)?;
        ending_last(&mut repeats, Verses::TWO, 8..12)?;
        // build
        let repeats = repeats.build()?;
        assert_eq!(repeats.repeats, vec![
            Repeat { verse: Some(0), bars: 0..4 },
            Repeat { verse: None, bars: 4..8 },
            Repeat { verse: Some(1), bars: 0..4 },
            Repeat { verse: None, bars: 8..12 },
        ]);
        Ok(())
    }

    #[test]
    fn test_1st_2nd_3rd_time() -> Result<()> {
        let mut repeats = new(2)?;
        repeat_start(&mut repeats, 2..4)?;
        ending(&mut repeats, Verses::ONE, 4..6)?;
        ending(&mut repeats, Verses::TWO, 6..8)?;
        ending_last(&mut repeats, Verses::THREE, 8..10)?;
        normal(&mut repeats, 10..12)?;
        // build
        let repeats = repeats.build()?;
        assert_eq!(repeats.repeats, vec![
            Repeat { verse: None, bars: 0..2 },
            Repeat { verse: Some(0), bars: 2..4 },
            Repeat { verse: None, bars: 4..6 },
            Repeat { verse: Some(1), bars: 2..4 },
            Repeat { verse: None, bars: 6..8 },
            Repeat { verse: Some(2), bars: 2..4 },
            Repeat { verse: None, bars: 8..10 },
            Repeat { verse: None, bars: 10..12 },
        ]);
        Ok(())
    }

    #[test]
    fn test_odd_even_5th_time() -> Result<()> {
        let mut repeats = new(2)?;
        repeat_start(&mut repeats, 2..4)?;
        ending(&mut repeats, Verses::ONE_THREE, 4..6)?;
        ending(&mut repeats, Verses::TWO_FOUR, 6..8)?;
        ending_last(&mut repeats, Verses::FIVE, 8..10)?;
        normal(&mut repeats, 10..12)?;
        // build
        let repeats = repeats.build()?;
        assert_eq!(repeats.repeats, vec![
            Repeat { verse: None, bars: 0..2 },
            Repeat { verse: Some(0), bars: 2..4 },
            Repeat { verse: Some(0), bars: 4..6 },
            Repeat { verse: Some(1), bars: 2..4 },
            Repeat { verse: Some(0), bars: 6..8 },
            Repeat { verse: Some(2), bars: 2..4 },
            Repeat { verse: Some(1), bars: 4..6 },
            Repeat { verse: Some(3), bars: 2..4 },
            Repeat { verse: Some(1), bars: 6..8 },
            Repeat { verse: Some(4), bars: 2..4 },
            Repeat { verse: None, bars: 8..10 },
            Repeat { verse: None, bars: 10..12 },
        ]);
        Ok(())
    }
}
