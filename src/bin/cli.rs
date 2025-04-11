use anyhow::Result;
use askama::Template;

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

                            Diff::Replace(replace) =>
                                println!("    {} <=> {}", replace.script, replace.music),
                        }
                    }
                }
            }
        }

        ["music", file] => {
            let xml = std::fs::read_to_string(file)?;
            let part = lyric_check::music::read(&xml)?;
            for phrase in &part.phrases {
                for word in &phrase.words {
                    print!("{} ", word.text);
                    if !word.phrases.is_empty() {
                        for phrase in &word.phrases {
                            println!("<<");
                            print!(" -->");
                            for word in &phrase.words {
                                print!("{} ", word.text);
                            }
                            print!(">>");
                        }
                    }
                }
                println!("");
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

