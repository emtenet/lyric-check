
pub mod diff;
pub mod music;
pub mod script;

#[derive(Debug)]
pub struct Section {
    pub heading: String,
    pub lines: Vec<Line>,
}

#[derive(Debug)]
pub struct Line {
    pub number: String,
    pub diffs: Vec<Diff>,
}

#[derive(Debug)]
pub enum Diff {
    Same(String),
    Music(String),
    Script(String),
    Replace(Replace),
}

#[derive(Debug)]
pub struct Replace {
    pub music: String,
    pub script: String,
}

