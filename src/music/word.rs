
use super::{
    Part,
    Phrase,
    syllable::Kind,
    Syllable,
    Word,
};

pub struct Builder {
    phrases: Vec<Phrase>,
    phrase: Phrase,
    word: Option<Word>,
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

    fn word(&mut self, word: Word) {
        let is_end = word.text.ends_with('.') || word.text.ends_with('!');
        if !self.phrase.words.is_empty() && word.start >= self.phrase.end + 512
        {
            // minum
            self.phrases.push(std::mem::replace(&mut self.phrase, Phrase {
                start: 0,
                end: 0,
                words: Vec::new(),
            }));
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

