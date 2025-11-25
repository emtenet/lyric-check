use anyhow::Result;
use askama::Template;
use std::path::PathBuf;

use lyric_check::{
    Diff,
    DiffPage,
};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_ref).collect();
    match &args[..] {
        ["html", music, script] => {
            let xml = std::fs::read_to_string(music)?;
            let txt = std::fs::read_to_string(script)?;
            let page = DiffPage {
                error: None,
                folder: String::new(),
                sections: lyric_check::diff::read(&txt, &xml)?,
            };
            let html = page.render()?;
            println!("{html}");
        }

        ["diff", music, script] => {
            let xml = std::fs::read_to_string(music)?;
            let txt = std::fs::read_to_string(script)?;
            let sections = lyric_check::diff::read(&txt, &xml)?;
            for section in sections {
                println!("{}", section.heading);
                for line in section.lines {
                    println!("{}", line.number);
                    for diff in line.diffs {
                        match diff {
                            Diff::Same(text) =>
                                println!("    {text}"),

                            Diff::Music(text) =>
                                println!("    ==> {text}"),

                            Diff::Script(text) =>
                                println!("    {text} <=="),

                            Diff::Case(text) =>
                                println!("    << {text} >>"),

                            Diff::Replace(replace) =>
                                println!("    {} <=> {}", replace.script, replace.music),
                        }
                    }
                }
            }
        }

        ["music", "--folder", folder, "--from", from] =>
            music_folder(folder, Some(from))?,

        ["music", "--folder", folder] =>
            music_folder(folder, None)?,

        ["music", file] => {
            let xml = std::fs::read_to_string(file)?;
            if let Some(music) = lyric_check::music::read(&xml)? {
                if let Some(title) = music.title {
                    println!(" {title}");
                    println!("{}", "=".repeat(2 + title.len()));
                    println!("");
                }
                for phrase in &music.phrases {
                    for word in &phrase.words {
                        print!("{} ", word.text);
                    }
                    println!("");
                }
            }
        }

        ["script", file] => {
            let txt = std::fs::read_to_string(file)?;
            let words = lyric_check::script::read(&txt)?;
            for word in words {
                println!("{word:?}");
            }
        }

        _ => {
            println!("lyric-check {args:?}");
        }
    }
    Ok(())
}

fn music_folder(folder: &str, from: Option<&str>) -> Result<()> {
    use std::io::Write;

    let folder = music_folder_list(folder)?;
    for (name, path) in folder {
        if let Some(from) = &from {
            if name.as_str() < from {
                continue;
            }
        }
        println!(" ==> {name}");
        let xml = std::fs::read_to_string(&path)?;
        if let Some(music) = lyric_check::music::read(&xml)? {
            let mut file = std::fs::File::create(path.with_extension("txt"))?;
            let title = music.title.unwrap_or(name);
            writeln!(file, " {title}")?;
            writeln!(file, "{}", "=".repeat(2 + title.len()))?;
            writeln!(file, "")?;
            for phrase in &music.phrases {
                for word in &phrase.words {
                    write!(file, "{} ", word.text)?;
                }
                writeln!(file, "")?;
            }
        }
    }
    Ok(())
}

fn music_folder_list(folder: &str) -> Result<Vec<(String, PathBuf)>> {
    let mut files = Vec::new();

    for entry in std::fs::read_dir(folder)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(name) = name.strip_suffix(".musicxml") {
            files.push((name.to_owned(), path));
        }
    }
    files.sort();
    Ok(files)
}
