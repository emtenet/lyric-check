use askama::Template;
use rocket::{
    config::{
        Config,
        LogLevel,
    },
    get,
    launch,
    routes,
    State,
};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;
use tokio::sync::Mutex;
use webbrowser;

use lyric_check::{
    HomePage,
};

mod asset;

use asset::Asset;

struct AppInner {
    root: PathBuf,
}

struct App(Mutex<AppInner>);

#[get("/")]
async fn index(app: &State<App>) -> String {
    let app = app.0.lock().await;

    let home = HomePage {
        root: String::from(app.root.to_str().unwrap_or("")),
    };
    home.render().unwrap()
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
    let app = App(Mutex::new(AppInner { root }));

    thread::spawn(move || {
        thread::sleep(Duration::from_secs(1));
        webbrowser::open("http://localhost:7000").unwrap();
    });

    let mut config = Config::release_default();
    config.log_level = LogLevel::Normal;
    // config.address = "localhost";
    config.port = 7000;

    rocket::custom(config)
        .manage(app)
        .mount("/", routes![
            index,
            index_css,
            favicon,
            favicon16,
            favicon32,
        ])
}


