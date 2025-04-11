use rocket::{
    http::ContentType,
    request::Request,
    response::{
        self,
        Responder,
        Response,
    },
};
use rust_embed::RustEmbed;
use std::borrow::Cow;
use std::io::Cursor;

#[derive(RustEmbed)]
#[folder = "asset"]
struct Assets;

pub struct Asset {
    content_type: ContentType,
    body: Cow<'static, [u8]>,
}

impl<'r, 'o: 'r> Responder<'r, 'o> for Asset {
    fn respond_to(self, _req: &'r Request<'_>) -> response::Result<'o> {
        Response::build()
            .header(self.content_type)
            .sized_body(self.body.len(), Cursor::new(self.body))
            .ok()
    }
}

impl Asset {
    fn get_typed(file: &str, content_type: ContentType) -> Option<Asset> {
        Assets::get(file).map(|f| Asset { content_type, body: f.data })
    }

    pub fn css(file: &str) -> Option<Asset> {
        Asset::get_typed(file, ContentType::CSS)
    }

    //pub fn html(file: &str) -> Option<Asset> {
    //    Asset::get_typed(file, ContentType::HTML)
    //}

    pub fn icon(file: &str) -> Option<Asset> {
        Asset::get_typed(file, ContentType::Icon)
    }

    //pub fn js(file: &str) -> Option<Asset> {
    //    Asset::get_typed(file, ContentType::JavaScript)
    //}

    pub fn png(file: &str) -> Option<Asset> {
        Asset::get_typed(file, ContentType::PNG)
    }
}

