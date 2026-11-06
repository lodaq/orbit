#[path = "backup.rs"]
mod backup;
pub use backup::transfer_data;

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::PathBuf;
pub(crate) fn config_path() -> io::Result<PathBuf> {
    #[cfg(windows)]
    let directory = PathBuf::from(
        std::env::var_os("APPDATA").ok_or_else(|| io::Error::other("APPDATA is not set"))?,
    );
    #[cfg(not(windows))]
    let directory = match std::env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty()) {
        Some(path) => PathBuf::from(path),
        None => PathBuf::from(
            std::env::var_os("HOME").ok_or_else(|| io::Error::other("HOME is not set"))?,
        )
        .join(".config"),
    };
    Ok(directory.join("orbit").join("config.json"))
}

pub fn read_config() -> io::Result<Option<Vec<u8>>> {
    match fs::read(config_path()?) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

pub fn backup_config() -> io::Result<()> {
    let path = config_path()?;
    fs::rename(&path, path.with_extension("json.bad"))
}

pub fn write_config(data: &[u8]) -> io::Result<()> {
    let path = config_path()?;
    fs::create_dir_all(path.parent().expect("config has a directory"))?;
    let temporary = path.with_extension("json.tmp");
    let result = (|| {
        let mut file = File::create(&temporary)?;
        file.write_all(data)?;
        file.sync_all()?;
        fs::rename(&temporary, &path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

fn recent_path() -> io::Result<PathBuf> {
    Ok(config_path()?.with_file_name("recent.json"))
}

pub fn read_recent() -> io::Result<Option<Vec<u8>>> {
    match fs::read(recent_path()?) {
        Ok(data) => Ok(Some(data)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

pub fn backup_recent() -> io::Result<()> {
    let path = recent_path()?;
    fs::rename(&path, path.with_extension("json.bad"))
}

pub fn write_recent(data: &[u8]) -> io::Result<()> {
    let path = recent_path()?;
    fs::create_dir_all(path.parent().expect("recent has a directory"))?;
    let temporary = path.with_extension("json.tmp");
    let result = (|| {
        let mut file = File::create(&temporary)?;
        file.write_all(data)?;
        file.sync_all()?;
        fs::rename(&temporary, &path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

fn installed_apps_path() -> io::Result<PathBuf> {
    Ok(config_path()?.with_file_name("installed-apps.json"))
}

pub fn read_installed_apps() -> io::Result<Option<Vec<u8>>> {
    match fs::read(installed_apps_path()?) {
        Ok(data) => Ok(Some(data)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

pub fn backup_installed_apps() -> io::Result<()> {
    let path = installed_apps_path()?;
    fs::rename(&path, path.with_extension("json.bad"))
}

pub fn write_installed_apps(data: &[u8]) -> io::Result<()> {
    let path = installed_apps_path()?;
    fs::create_dir_all(path.parent().expect("installed apps have a directory"))?;
    let temporary = path.with_extension("json.tmp");
    let result = (|| {
        let mut file = File::create(&temporary)?;
        file.write_all(data)?;
        file.sync_all()?;
        fs::rename(&temporary, &path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

fn icons_directory() -> io::Result<PathBuf> {
    let directory = config_path()?
        .parent()
        .expect("config has a directory")
        .join("icons");
    fs::create_dir_all(&directory)?;
    Ok(directory)
}

pub fn icon_path(name: &str) -> io::Result<PathBuf> {
    Ok(icons_directory()?.join(name))
}

pub fn read_icon(name: &str) -> io::Result<Vec<u8>> {
    fs::read(icon_path(name)?)
}

pub fn write_icon(name: &str, data: &[u8]) -> io::Result<()> {
    let path = icon_path(name)?;
    let temporary = path.with_extension("tmp");
    let result = (|| {
        let mut file = File::create(&temporary)?;
        file.write_all(data)?;
        file.sync_all()?;
        fs::rename(&temporary, &path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

pub fn remove_icon(name: &str) {
    if let Ok(path) = icon_path(name) {
        let _ = fs::remove_file(path);
    }
}

type FetchedIcon = (
    (String, Vec<u8>),
    slint::SharedPixelBuffer<slint::Rgba8Pixel>,
);

pub fn fetch_icon(url: &str) -> Option<FetchedIcon> {
    let uri = website_url(url)?;
    let origin = format!("{}://{}", uri.scheme_str()?, uri.authority()?);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(6);
    let (sender, receiver) = std::sync::mpsc::channel();
    let website = url.to_string();
    let direct = sender.clone();
    std::thread::spawn(move || {
        let _ = direct.send((true, fetch_site_icon(&website, deadline)));
    });
    // A blocked website must not delay the independent fallback by several timeouts.
    std::thread::spawn(move || {
        let address = format!("https://www.google.com/s2/favicons?domain_url={origin}&sz=256");
        let icon = fetch_url(&address, deadline).and_then(|(_, data)| decode_fetched_icon(data));
        let _ = sender.send((false, icon));
    });
    let mut best = None;
    while let Ok((direct, icon)) =
        receiver.recv_timeout(deadline.saturating_duration_since(std::time::Instant::now()))
    {
        if let Some(icon) = icon {
            let ready = direct && icon_quality(&icon) >= 128;
            keep_better_icon(&mut best, icon);
            if ready {
                break;
            }
        }
    }
    best
}

fn fetch_site_icon(url: &str, deadline: std::time::Instant) -> Option<FetchedIcon> {
    let mut addresses = Vec::new();
    let mut base = url.to_string();
    if let Some((effective_url, page)) = fetch_url(url, deadline) {
        addresses = icon_links(&page, &effective_url);
        base = effective_url;
    }
    let uri = website_url(&base)?;
    let favicon = format!("{}://{}/favicon.ico", uri.scheme_str()?, uri.authority()?);
    // shortcut: try the three best declarations, expand only if sites need more broken-icon fallbacks.
    addresses.truncate(3);
    let mut best = None;
    for address in addresses {
        if let Some((_, data)) = fetch_url(&address, deadline)
            && let Some(icon) = decode_fetched_icon(data)
        {
            let ready = icon_quality(&icon) >= 128;
            keep_better_icon(&mut best, icon);
            if ready {
                break;
            }
        }
    }
    best.or_else(|| fetch_url(&favicon, deadline).and_then(|(_, data)| decode_fetched_icon(data)))
}

fn icon_quality(icon: &FetchedIcon) -> u32 {
    if icon.0.0 == "svg" {
        u32::MAX
    } else {
        icon.1.width().min(icon.1.height())
    }
}

fn keep_better_icon(best: &mut Option<FetchedIcon>, icon: FetchedIcon) {
    if best
        .as_ref()
        .is_none_or(|current| icon_quality(&icon) > icon_quality(current))
    {
        *best = Some(icon);
    }
}

fn decode_fetched_icon(data: Vec<u8>) -> Option<FetchedIcon> {
    let extension = ["svg", "png", "jpg", "ico"]
        .into_iter()
        .find(|extension| is_icon(&data, extension))?;
    let preview = crate::apps::decode_icon(&data, &format!("preview.{extension}")).ok()?;
    (preview.width() > 0 && preview.height() > 0).then_some(((extension.into(), data), preview))
}

fn fetch_url(address: &str, deadline: std::time::Instant) -> Option<(String, Vec<u8>)> {
    use std::time::{Duration, Instant};
    use ureq::ResponseExt;
    website_url(address)?;
    let timeout = deadline.checked_duration_since(Instant::now())?;
    static AGENT: std::sync::OnceLock<ureq::Agent> = std::sync::OnceLock::new();
    let agent = AGENT.get_or_init(|| {
        ureq::Agent::config_builder()
            .max_redirects(5)
            .timeout_connect(Some(Duration::from_secs(2)))
            .user_agent("Orbit/0.1")
            .build()
            .into()
    });
    let mut response = agent
        .get(address)
        .config()
        .timeout_global(Some(timeout))
        .build()
        .call()
        .ok()?;
    let effective_url = response.get_uri().to_string();
    let data = response
        .body_mut()
        .with_config()
        .limit(2 * 1024 * 1024)
        .read_to_vec()
        .ok()?;
    Some((effective_url, data))
}

fn website_url(value: &str) -> Option<ureq::http::Uri> {
    let uri: ureq::http::Uri = value.parse().ok()?;
    (matches!(uri.scheme_str(), Some("http" | "https"))
        && !uri.authority()?.as_str().contains('@')
        && !uri.host()?.is_empty())
    .then_some(uri)
}

fn icon_links(html: &[u8], base: &str) -> Vec<String> {
    let html = String::from_utf8_lossy(html);
    let lowercase = html.to_ascii_lowercase();
    let mut links = Vec::new();
    let mut offset = 0;
    while let Some(start) = lowercase[offset..].find("<link") {
        let start = offset + start;
        let Some(end) = html[start..].find('>') else {
            break;
        };
        offset = start + end + 1;
        let tag = &html[start..start + end];
        let Some(rel) = attribute(tag, "rel") else {
            continue;
        };
        let touch = rel.split_ascii_whitespace().any(|word| {
            word.eq_ignore_ascii_case("apple-touch-icon")
                || word.eq_ignore_ascii_case("apple-touch-icon-precomposed")
        });
        if !touch
            && !rel
                .split_ascii_whitespace()
                .any(|word| word.eq_ignore_ascii_case("icon"))
        {
            continue;
        }
        let Some(href) = attribute(tag, "href") else {
            continue;
        };
        let Some(address) = resolve_icon_url(base, &href.replace("&amp;", "&")) else {
            continue;
        };
        let size = attribute(tag, "sizes")
            .unwrap_or("")
            .split_ascii_whitespace()
            .filter_map(|size| {
                if size.eq_ignore_ascii_case("any") {
                    return Some(u32::MAX);
                }
                let (width, height) = size.split_once(['x', 'X'])?;
                Some(width.parse::<u32>().ok()?.min(height.parse().ok()?))
            })
            .max()
            .unwrap_or(if touch { 180 } else { 0 });
        let vector = attribute(tag, "type")
            .is_some_and(|kind| kind.eq_ignore_ascii_case("image/svg+xml"))
            || address
                .split(['?', '#'])
                .next()
                .is_some_and(|path| path.to_ascii_lowercase().ends_with(".svg"));
        links.push((if vector { u32::MAX } else { size }, address));
    }
    links.sort_by_key(|link| std::cmp::Reverse(link.0));
    let mut addresses = Vec::new();
    for (_, address) in links {
        if !addresses.contains(&address) {
            addresses.push(address);
        }
    }
    addresses
}

fn resolve_icon_url(base: &str, href: &str) -> Option<String> {
    let base = website_url(base)?;
    let href = href.trim().split('#').next()?;
    if href.is_empty() || href.starts_with('#') {
        return None;
    }
    let address = if href.starts_with("//") {
        format!("{}:{href}", base.scheme_str()?)
    } else if href.contains(':') {
        href.to_string()
    } else {
        let origin = format!("{}://{}", base.scheme_str()?, base.authority()?);
        let path = if href.starts_with('/') {
            href.to_string()
        } else if href.starts_with('?') {
            format!("{}{href}", base.path())
        } else {
            format!(
                "{}/{href}",
                base.path()
                    .rsplit_once('/')
                    .map_or("", |(parent, _)| parent)
            )
        };
        let end = path.find(['?', '#']).unwrap_or(path.len());
        let mut parts = Vec::new();
        for part in path[..end].split('/') {
            match part {
                "" | "." => {}
                ".." => {
                    parts.pop();
                }
                _ => parts.push(part),
            }
        }
        format!("{origin}/{}{}", parts.join("/"), &path[end..])
    };
    website_url(&address).map(|_| address)
}

fn attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let bytes = tag.as_bytes();
    let mut offset = 5;
    while offset < bytes.len() {
        while offset < bytes.len() && (bytes[offset].is_ascii_whitespace() || bytes[offset] == b'/')
        {
            offset += 1;
        }
        let start = offset;
        while offset < bytes.len() && !bytes[offset].is_ascii_whitespace() && bytes[offset] != b'='
        {
            offset += 1;
        }
        if start == offset {
            break;
        }
        let key = &tag[start..offset];
        while offset < bytes.len() && bytes[offset].is_ascii_whitespace() {
            offset += 1;
        }
        if offset == bytes.len() || bytes[offset] != b'=' {
            continue;
        }
        offset += 1;
        while offset < bytes.len() && bytes[offset].is_ascii_whitespace() {
            offset += 1;
        }
        if offset == bytes.len() {
            break;
        }
        let quote = matches!(bytes[offset], b'"' | b'\'').then_some(bytes[offset]);
        if quote.is_some() {
            offset += 1;
        }
        let start = offset;
        while offset < bytes.len()
            && quote.map_or(!bytes[offset].is_ascii_whitespace(), |quote| {
                bytes[offset] != quote
            })
        {
            offset += 1;
        }
        let value = &tag[start..offset];
        if quote.is_some() && offset < bytes.len() {
            offset += 1;
        }
        if key.eq_ignore_ascii_case(name) {
            return Some(value);
        }
    }
    None
}

pub fn choose_path(directory: bool) -> Option<PathBuf> {
    let title = if directory {
        "Choose directory"
    } else {
        "Choose file"
    };
    let _guard = crate::platform::float_file_chooser(title);
    let dialog = rfd::FileDialog::new().set_title(title);
    if directory {
        dialog.pick_folder()
    } else {
        dialog.pick_file()
    }
}

fn is_icon(data: &[u8], extension: &str) -> bool {
    match extension {
        "svg" => {
            let data = data.trim_ascii_start();
            data.starts_with(b"<svg") || data.starts_with(b"<?xml")
        }
        "png" => data.starts_with(b"\x89PNG\r\n\x1a\n"),
        "jpg" => data.starts_with(b"\xff\xd8\xff"),
        "ico" => data.starts_with(&[0, 0, 1, 0]),
        _ => false,
    }
}

pub fn choose_icon() -> Option<PathBuf> {
    let _guard = crate::platform::float_file_chooser("Choose icon");
    rfd::FileDialog::new()
        .set_title("Choose icon")
        .add_filter("Icon", &["png", "jpg", "jpeg", "svg", "ico"])
        .pick_file()
}

pub fn choose_background_image() -> Option<PathBuf> {
    let _guard = crate::platform::float_file_chooser("Choose background image");
    rfd::FileDialog::new()
        .set_title("Choose background image")
        .add_filter("Image", &["png", "jpg", "jpeg", "svg"])
        .pick_file()
}

pub fn read_chosen_icon(value: &str) -> Result<(String, Vec<u8>), String> {
    use std::hash::{Hash, Hasher};
    let path = PathBuf::from(value);
    let data = if path.is_absolute() {
        fs::read(&path)
    } else {
        read_icon(value)
    }
    .map_err(|_| "Couldn't read icon")?;
    let extension = path
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or("")
        .to_lowercase();
    if !matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "svg" | "ico") {
        return Err("Choose a PNG, JPEG, SVG or ICO icon.".into());
    }
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    data.hash(&mut hash);
    Ok((format!("chosen-{:x}.{extension}", hash.finish()), data))
}

#[cfg(test)]
mod icon_tests {
    use super::*;

    #[test]
    fn prefers_large_touch_icons_over_small_favicons() {
        assert_eq!(
            icon_links(
                br#"<link rel="icon" sizes="16x16" href="/small.png"><link rel="apple-touch-icon" sizes="180x180" href="/large.png">"#,
                "https://example.com",
            ),
            vec!["https://example.com/large.png".to_string(), "https://example.com/small.png".to_string()],
        );
    }

    #[test]
    fn ranks_vectors_and_large_icons_and_removes_duplicates() {
        let links = icon_links(
            br#"
            <LINK HREF="small.png" SIZES="16x16" REL="ICON">
            <link href="large.png" sizes="48x48 256x256" rel="icon">
            <link href="vector" type="image/svg+xml" rel="icon">
            <link href="large.png" sizes="128x128" rel="icon">
            <link href="data:image/png;base64,AAAA" rel="icon">
        "#,
            "https://example.com/sub/page",
        );
        assert_eq!(
            links,
            vec![
                "https://example.com/sub/vector",
                "https://example.com/sub/large.png",
                "https://example.com/sub/small.png",
            ]
        );
    }

    #[test]
    fn resolves_relative_and_protocol_relative_icon_urls() {
        for (href, expected) in [
            ("../icon.png", "https://example.com/icon.png"),
            ("../../icon.png", "https://example.com/icon.png"),
            ("./icon.png", "https://example.com/sub/icon.png"),
            (
                "//cdn.example.com/icon.png",
                "https://cdn.example.com/icon.png",
            ),
            ("/icon.svg#symbol", "https://example.com/icon.svg"),
            ("?icon=1", "https://example.com/sub/page?icon=1"),
        ] {
            assert_eq!(
                resolve_icon_url("https://example.com/sub/page", href).as_deref(),
                Some(expected)
            );
        }
        for href in [
            "data:image/png,abc",
            "file:///tmp/icon",
            "javascript:alert(1)",
            "#icon",
            "https://user:password@example.com/icon",
        ] {
            assert!(resolve_icon_url("https://example.com", href).is_none());
        }
    }

    #[test]
    fn rejects_undecodable_images_and_renders_small_vectors_sharply() {
        assert!(decode_fetched_icon(b"\x89PNG\r\n\x1a\nnot an image".to_vec()).is_none());
        let vector = decode_fetched_icon(br#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><rect width="16" height="16" fill="red"/></svg>"#.to_vec()).unwrap();
        assert_eq!((vector.1.width(), vector.1.height()), (80, 80));
        assert_eq!(icon_quality(&vector), u32::MAX);
    }

    #[test]
    fn follows_redirects_and_tries_the_next_icon_after_decode_failure() {
        use std::io::{BufRead, BufReader};
        use std::net::TcpListener;
        use std::time::{Duration, Instant};
        let mut png = std::io::Cursor::new(Vec::new());
        image::RgbaImage::from_pixel(256, 256, image::Rgba([0, 80, 180, 255]))
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        let png = png.into_inner();
        let expected = png.clone();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/start", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            for (path, status, headers, body) in [
                ("/start", "302 Found", "Location: /sub/page\r\n", Vec::new()),
                (
                    "/sub/page",
                    "200 OK",
                    "",
                    br#"
                    <!doctype html><link rel="icon" type="image/svg+xml" href="../broken.svg">
                    <link rel="apple-touch-icon" sizes="256x256" href="../image?x=1&amp;y=2">
                    <link rel="icon" sizes="16x16" href="/small.png">
                "#
                    .to_vec(),
                ),
                (
                    "/broken.svg",
                    "200 OK",
                    "",
                    b"<?xml version='1.0'?><error/>".to_vec(),
                ),
                ("/image?x=1&y=2", "200 OK", "", png),
            ] {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                let mut reader = BufReader::new(&stream);
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                assert_eq!(line.split_whitespace().nth(1), Some(path));
                loop {
                    line.clear();
                    reader.read_line(&mut line).unwrap();
                    if line == "\r\n" || line.is_empty() {
                        break;
                    }
                }
                write!(
                    stream,
                    "HTTP/1.1 {status}\r\nConnection: close\r\nContent-Length: {}\r\n{headers}\r\n",
                    body.len()
                )
                .unwrap();
                stream.write_all(&body).unwrap();
            }
        });
        let icon = fetch_site_icon(&url, Instant::now() + Duration::from_secs(6)).unwrap();
        server.join().unwrap();
        assert_eq!(icon.0, ("png".into(), expected));
        assert_eq!((icon.1.width(), icon.1.height()), (256, 256));
    }

    #[test]
    fn request_deadline_stops_a_stalled_server() {
        use std::net::TcpListener;
        use std::time::{Duration, Instant};
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = format!("http://{}/", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let (_stream, _) = listener.accept().unwrap();
            std::thread::sleep(Duration::from_millis(400));
        });
        let start = Instant::now();
        assert!(fetch_url(&address, start + Duration::from_millis(100)).is_none());
        assert!(start.elapsed() < Duration::from_millis(350));
        server.join().unwrap();
    }

    #[test]
    #[ignore = "requires public website and Google connectivity"]
    fn fetches_live_website_icons() {
        for url in [
            "https://www.linkedin.com",
            "https://github.com",
            "https://www.wikipedia.org",
        ] {
            let start = std::time::Instant::now();
            let icon = fetch_icon(url).unwrap_or_else(|| panic!("No icon for {url}"));
            eprintln!(
                "{url}: {} {}x{} in {:.2}s",
                icon.0.0,
                icon.1.width(),
                icon.1.height(),
                start.elapsed().as_secs_f32()
            );
            assert!(start.elapsed() < std::time::Duration::from_secs(7));
        }
    }

    #[test]
    fn reads_site_icon_with_attributes_in_either_order() {
        assert_eq!(
            icon_links(
                br#"<link rel="stylesheet" href="x.css"><link href="/brand.png" sizes="64x64" rel="icon">"#,
                "https://example.com",
            ),
            vec!["https://example.com/brand.png".to_string()],
        );
    }

    #[test]
    fn reads_shortcut_icon_rel_tokens() {
        assert_eq!(
            icon_links(
                br#"<link rel="shortcut icon" href="/favicon.png">"#,
                "https://example.com",
            ),
            vec!["https://example.com/favicon.png".to_string()],
        );
    }

    #[test]
    fn reads_absolute_site_icon_url() {
        assert_eq!(
            icon_links(
                br#"<link rel="icon" href="https://icons.example.com/brand.png">"#,
                "https://example.com",
            ),
            vec!["https://icons.example.com/brand.png".to_string()],
        );
    }
}
