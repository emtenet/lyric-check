use anyhow::Result;

#[derive(Debug)]
pub enum Word {
    Heading(String),
    Line(String),
    Word(String),
}

enum State {
    Start,
    Digit,
    Dot,
    Space,
    Word {
        from: usize,
    },
}

pub fn read<'str>(txt: &'str str) -> Result<Vec<Word>> {
    let mut words = Vec::new();

    for line in txt.lines() {
        let mut state = State::Start;
        let line = line
            .replace('\u{2019}', "\'")
            .replace('\u{2026}', "...");
        for (i, c) in line.char_indices() {
            if !c.is_ascii() {
               println!("NOT-ASCII `{c}` {:x}", u32::from(c));
            }
            match state {
                State::Start if c.is_ascii_digit() =>
                    state = State::Digit,
                State::Start =>
                    break,

                State::Digit if c.is_ascii_digit() =>
                    (),
                State::Digit if c == '.' =>
                    state = State::Dot,
                State::Digit => {
                    state = State::Start;
                    break;
                }

                State::Dot => {
                    words.push(Word::Line(String::from(&line[..i])));
                    if c == ' ' {
                        state = State::Space;
                    } else {
                        state = State::Word {
                            from: i,
                        };
                    }
                }

                State::Space if c == ' ' =>
                    (),
                State::Space =>
                    state = State::Word {
                        from: i,
                    },

                State::Word { from } if c == ' ' => {
                    words.push(Word::Word(String::from(&line[from..i])));
                    state = State::Space;
                }
                State::Word { .. } =>
                    (),
            }
        }
        match state {
            State::Start =>
                words.push(Word::Heading(String::from(line))),

            State::Word { from } =>
                words.push(Word::Word(String::from(&line[from..]))),

            _ =>
                (),
        }
    }

    Ok(words)
}

