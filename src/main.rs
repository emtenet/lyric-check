use anyhow::Result;

fn main() -> Result<()> {
    let mut args = std::env::args();
    let _ = args.next();
    if let Some(file) = args.next() {
        let xml = std::fs::read_to_string(file)?;
        let part = lyric_check::music::read(&xml)?;
        for phrase in &part.phrases {
            for word in &phrase.words {
                print!("{} ", word.text);
            }
            println!("");
        }
    }
    Ok(())
}

