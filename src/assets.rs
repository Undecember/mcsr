pub fn web_asset(path: &str) -> Option<(&'static [u8], &'static str)> {
    match path {
        "/" | "/index.html" => Some((
            include_bytes!("../static/index.html"),
            "text/html; charset=utf-8",
        )),
        "/app.js" => Some((
            include_bytes!("../static/app.js"),
            "text/javascript; charset=utf-8",
        )),
        "/style.css" => Some((
            include_bytes!("../static/style.css"),
            "text/css; charset=utf-8",
        )),
        _ => None,
    }
}

pub fn extension_archive() -> &'static [u8] {
    include_bytes!(concat!(env!("OUT_DIR"), "/extension.zip"))
}
