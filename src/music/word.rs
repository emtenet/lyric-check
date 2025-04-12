
use super::{
    Part,
    Phrase,
    syllable::Kind,
    Syllable,
    Word,
    MINIM,
};

pub struct Builder {
    phrases: Vec<Phrase>,
    phrase: Phrase,
    word: Option<Word>,
    in_group: bool,
}

impl Builder {
    pub fn new() -> Self {
        Builder {
            phrases: Vec::new(),
            phrase: Phrase {
                start: 0,
                end: 0,
                words: Vec::new(),
            },
            word: None,
            in_group: false,
        }
    }

    pub fn syllable(&mut self, syllable: Syllable) {
        match syllable.kind {
            Kind::Single => {
                if let Some(word) = self.word.take() {
                    self.word(word);
                }
                self.word(Word {
                    start: syllable.start,
                    end: syllable.end,
                    text: String::from(syllable.text),
                });
            }

            Kind::Begin => {
                if let Some(word) = self.word.take() {
                    self.word(word);
                }
                self.word = Some(Word {
                    start: syllable.start,
                    end: syllable.end,
                    text: String::from(syllable.text),
                });
            }

            Kind::Middle =>
                if let Some(word) = &mut self.word {
                    word.end = syllable.end;
                    word.text.push_str(syllable.text);
                } else {
                    self.word = Some(Word {
                        start: syllable.start,
                        end: syllable.end,
                        text: String::from(syllable.text),
                    });
                },

            Kind::End =>
                if let Some(mut word) = self.word.take() {
                    word.end = syllable.end;
                    word.text.push_str(syllable.text);
                    self.word(word);
                } else {
                    self.word(Word {
                        start: syllable.start,
                        end: syllable.end,
                        text: String::from(syllable.text),
                    });
                },
        }
    }

    fn word(&mut self, mut word: Word) {
        while let Some((left, right)) = word.text.split_once(' ') {
            self.word_single(Word {
                start: word.start,
                end: word.start + 1,
                text: String::from(left),
            });
            word.start += 1;
            word.text = String::from(right);
        }
        self.word_single(word);
    }

    fn word_single(&mut self, word: Word) {
        let group_start = word.text.starts_with('[');
        let group_end = word.text.ends_with(']');
        let is_end = if self.in_group {
            group_end
        } else {
            word.text.ends_with('.') || word.text.ends_with('!')
        };
        if !self.phrase.words.is_empty() {
            let new_phrase = if self.in_group {
                false
            } else {
                // start new phrase at a capital letter
                // and there is a rest between the previous word
                let rest_then_capital = is_capital(&word.text) && word.start > self.phrase.end;
                // OR
                // start new phrase after a big rest (minum)
                let big_rest = word.start >= self.phrase.end + MINIM;
                rest_then_capital || big_rest
            };
            if group_start || new_phrase {
                self.phrases.push(std::mem::replace(&mut self.phrase, Phrase {
                    start: 0,
                    end: 0,
                    words: Vec::new(),
                }));
            }
        }
        if self.phrase.words.is_empty() {
            if is_end {
                self.phrases.push(Phrase {
                    start: word.start,
                    end: word.end,
                    words: vec![word],
                });
            } else {
                self.phrase.start = word.start;
                self.phrase.end = word.end;
                self.phrase.words.push(word);
            }
        } else {
            self.phrase.end = word.end;
            self.phrase.words.push(word);
            if is_end {
                self.phrases.push(std::mem::replace(&mut self.phrase, Phrase {
                    start: 0,
                    end: 0,
                    words: Vec::new(),
                }));
            }
        }
        if group_start {
            self.in_group = true;
        } else if group_end {
            self.in_group = false;
        }
    }

    pub fn build(mut self) -> Part {
        if let Some(word) = self.word.take() {
            self.word(word);
        }
        if !self.phrase.words.is_empty() {
            self.phrases.push(self.phrase);
        }
        Part {
            phrases: self.phrases,
        }
    }
}

fn is_capital(text: &str) -> bool {
    let mut chars = text.chars();
    if let Some(c) = chars.next() {
        if c.is_ascii_uppercase() {
            return true;
        }
        if c == '\'' {
            if let Some(c) = chars.next() {
                if c.is_ascii_uppercase() {
                    return true;
                }
            }
        }
    }
    false
}