use anyhow::Result;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_ref).collect();
    match &args[..] {
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

