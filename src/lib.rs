use askama::Template;

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
    Case(String),
    Replace(Replace),
}

#[derive(Debug)]
pub struct Replace {
    pub music: String,
    pub script: String,
}

impl Replace {
    fn to_diff(self) -> Diff {
        if single_letter(&self.music) == single_letter(&self.script) {
            Diff::Case(self.script)
        } else {
            Diff::Replace(self)
        }
    }
}

fn single_letter(s: &str) -> Option<char> {
    let mut chars = s.chars();
    if let Some(c) = chars.next() {
        if chars.next().is_none() {
            return Some(c.to_ascii_lowercase());
        }
    }
    None
}

pub struct Link {
    pub selected: bool,
    pub href: String,
    pub title: String,
}

#[derive(askama::Template)]
#[template(path = "home.html")]
pub struct HomePage {
    pub error: Option<String>,
    pub folders: Vec<Link>,
    pub scripts: Vec<Link>,
    pub musics: Vec<Link>,
}

#[derive(askama::Template)]
#[template(path = "folder.html")]
pub struct FolderPage {
    pub error: Option<String>,
    pub scripts: Vec<Link>,
    pub musics: Vec<Link>,
}

#[derive(askama::Template)]
#[template(path = "diff.html")]
pub struct DiffPage {
    pub error: Option<String>,
    pub sections: Vec<Section>,
}

#[derive(askama::Template)]
#[template(path = "error.html")]
pub struct ErrorPage {
    pub error: Option<String>,
}

impl ErrorPage {
    pub fn anyhow(error: anyhow::Error) -> String {
        let page = ErrorPage {
            error: Some(format!("{error:?}")),
        };
        page.render().unwrap()
    }
}