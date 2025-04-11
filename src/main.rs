use anyhow::Context;
use askama::Template;
use rocket::{
    config::{
        Config,
        LogLevel,
    },
    get,
    launch,
    response::content::RawHtml,
    routes,
    State,
};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;
use webbrowser;

use lyric_check::{
    DiffPage,
    ErrorPage,
    FolderPage,
    HomePage,
    Link,
};

mod asset;

use asset::Asset;

#[get("/")]
async fn html_root(root: &State<PathBuf>) -> RawHtml<String> {
    match page_root(&root).await {
        Ok(html) =>
            RawHtml(html),

        Err(error) =>
            RawHtml(ErrorPage::anyhow(error)),
    }
}

#[get("/folder/<folder>")]
async fn html_folder(root: &State<PathBuf>, folder: &str) -> RawHtml<String> {
    match page_folder(&root, folder, None, None).await {
        Ok(html) =>
            RawHtml(html),

        Err(error) =>
            RawHtml(ErrorPage::anyhow(error)),
    }
}

#[get("/folder/<folder>/script/<script>")]
async fn html_folder_script(
    root: &State<PathBuf>,
    folder: &str, 
    script: String,
) -> RawHtml<String> {
    match page_folder(&root, folder, Some(script), None).await {
        Ok(html) =>
            RawHtml(html),

        Err(error) =>
            RawHtml(ErrorPage::anyhow(error)),
    }
}

#[get("/folder/<folder>/script/<script>/music/<music>")]
async fn html_folder_script_music(
    root: &State<PathBuf>,
    folder: &str, 
    script: String,
    music: String,
) -> RawHtml<String> {
    match page_folder(&root, folder, Some(script), Some(music)).await {
        Ok(html) =>
            RawHtml(html),

        Err(error) =>
            RawHtml(ErrorPage::anyhow(error)),
    }
}

#[get("/folder/<folder>/music/<music>")]
async fn html_folder_music(
    root: &State<PathBuf>,
    folder: &str, 
    music: String,
) -> RawHtml<String> {
    match page_folder(&root, folder, None, Some(music)).await {
        Ok(html) =>
            RawHtml(html),

        Err(error) =>
            RawHtml(ErrorPage::anyhow(error)),
    }
}

#[get("/folder/<folder>/script/<script>/music/<music>/diff")]
async fn html_folder_diff(
    root: &State<PathBuf>,
    folder: &str,
    script: &str,
    music: &str,
) -> RawHtml<String> {
    match page_folder_diff(&root, folder, script, music).await {
        Ok(html) =>
            RawHtml(html),

        Err(error) =>
            RawHtml(ErrorPage::anyhow(error)),
    }
}

#[get("/index.css")]
fn index_css() -> Option<Asset> {
    Asset::css("index.css")
}

#[get("/favicon.ico")]
fn favicon() -> Option<Asset> {
    Asset::icon("favicon.ico")
}

#[get("/favicon-16x16.png")]
fn favicon16() -> Option<Asset> {
    Asset::png("favicon-16x16.png")
}

#[get("/favicon-32x32.png")]
fn favicon32() -> Option<Asset> {
    Asset::png("favicon-32x32.png")
}

#[launch]
async fn rocket() -> _ {
    let mut root = std::env::current_dir().unwrap();
    if let Some(arg) = std::env::args().skip(1).next() {
        let arg = Path::new(&arg);
        if let Ok(meta) = std::fs::metadata(arg) {
            if meta.is_dir() {
                root = PathBuf::from(arg);
            }
        }
    }

    thread::spawn(move || {
        thread::sleep(Duration::from_secs(1));
        webbrowser::open("http://localhost:7000").unwrap();
    });

    let mut config = Config::release_default();
    config.log_level = LogLevel::Normal;
    // config.address = "localhost";
    config.port = 7000;

    rocket::custom(config)
        .manage(root)
        .mount("/", routes![
            html_root,
            html_folder,
            html_folder_script,
            html_folder_script_music,
            html_folder_music,
            html_folder_diff,
            index_css,
            favicon,
            favicon16,
            favicon32,
        ])
}

struct Home {
    folders: Vec<String>,
    scripts: Vec<String>,
    musics: Vec<String>,
}

impl Home {
    async fn read(root: &Path) -> anyhow::Result<Self> {
        let mut answer = Home {
            folders: Vec::new(),
            scripts: Vec::new(),
            musics: Vec::new(),
        };

        let mut dir = tokio::fs::read_dir(root).await?;
        while let Some(entry) = dir.next_entry().await? {
            if let Some(name) = entry.file_name().to_str() {
                let file_type = entry.file_type().await?;
                if file_type.is_dir() {
                    answer.folders.push(String::from(name));
                } else if file_type.is_file() {
                    if let Some(name) = name.strip_suffix(".txt") {
                        answer.scripts.push(String::from(name));
                    }
                    if let Some(name) = name.strip_suffix(".musicxml") {
                        answer.musics.push(String::from(name));
                    }
                }
            }
        }

        answer.folders.sort();
        answer.scripts.sort();
        answer.musics.sort();

        Ok(answer)
    }
}

struct Folder {
    scripts: Vec<String>,
    musics: Vec<String>,
}

impl Folder {
    async fn read(root: &Path, folder: &str) -> anyhow::Result<Self> {
        let mut answer = Folder {
            scripts: Vec::new(),
            musics: Vec::new(),
        };

        let root = root.join(folder);
        let mut dir = tokio::fs::read_dir(root).await?;
        while let Some(entry) = dir.next_entry().await? {
            if let Some(name) = entry.file_name().to_str() {
                let file_type = entry.file_type().await?;
                if file_type.is_file() {
                    if let Some(name) = name.strip_suffix(".txt") {
                        answer.scripts.push(String::from(name));
                    }
                    if let Some(name) = name.strip_suffix(".musicxml") {
                        answer.musics.push(String::from(name));
                    }
                }
            }
        }

        answer.scripts.sort();
        answer.musics.sort();

        Ok(answer)
    }
}

async fn page_root(root: &Path) -> anyhow::Result<String> {
    let dir = Home::read(root).await?;
    let page = HomePage {
        error: None,
        folders: dir.folders.into_iter().map(|folder|
            Link {
                selected: false,
                href: format!("/folder/{folder}"),
                title: folder,
            }
        ).collect(),
        scripts: dir.scripts.into_iter().map(|script|
            Link {
                selected: false,
                href: format!("/script/{script}"),
                title: script,
            }
        ).collect(),
        musics: dir.musics.into_iter().map(|music|
            Link {
                selected: false,
                href: format!("/music/{music}"),
                title: music,
            }
        ).collect(),
    };
    Ok(page.render().unwrap())
}

async fn page_folder(
    root: &Path,
    folder: &str,
    mut selected_script: Option<String>,
    mut selected_music: Option<String>,
) -> anyhow::Result<String> {
    let dir = Folder::read(root, folder).await?;
    if let [script] = &dir.scripts[..] {
        selected_script = Some(script.clone());
    }
    if let [music] = &dir.musics[..] {
        selected_music = Some(music.clone());
    }
    let page = FolderPage {
        error: None,
        scripts: dir.scripts.into_iter().map(|script|
            Link {
                selected: if let Some(ref selected) = selected_script {
                    selected == &script
                } else {
                    false
                },
                href: if let Some(ref music) = selected_music {
                    format!("/folder/{folder}/script/{script}/music/{music}/diff")
                } else {
                    format!("/folder/{folder}/script/{script}")
                },
                title: script,
            }
        ).collect(),
        musics: dir.musics.into_iter().map(|music|
            Link {
                selected: if let Some(ref selected) = selected_music {
                    selected == &music
                } else {
                    false
                },
                href: if let Some(ref script) = selected_script {
                    format!("/folder/{folder}/script/{script}/music/{music}/diff")
                } else {
                    format!("/folder/{folder}/music/{music}")
                },
                title: music,
            }
        ).collect(),
    };
    Ok(page.render().unwrap())
}

async fn page_folder_diff(
    root: &Path,
    folder: &str,
    script: &str,
    music: &str,
) -> anyhow::Result<String> {
    let folder = root.join(folder);

    let script = format!("{script}.txt");
    let script = folder.join(script);
    let script = tokio::fs::read_to_string(&script).await
        .with_context(|| format!("Read from {}", script.display()))?;

    let music = format!("{music}.musicxml");
    let music = folder.join(music);
    let music = tokio::fs::read_to_string(&music).await
        .with_context(|| format!("Read from {}", music.display()))?;

    let sections = lyric_check::diff::read(&script, &music)?;
    let page = DiffPage {
        error: None,
        sections,
    };
    Ok(page.render().unwrap())
}
