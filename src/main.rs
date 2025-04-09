use anyhow::Result;

fn main() -> Result<()> {
    let mut args = std::env::args();
    let _ = args.next();
    if let Some(file) = args.next() {
        let xml = std::fs::read_to_string(file)?;
        lyric_check::music::read(&xml)?;
    }
    Ok(())
}

