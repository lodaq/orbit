use crate::search_paths::url_encode;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::{Duration, Instant};

const LENS_UPLOAD_URL: &str = "https://lens.google.com/v3/upload?hl=en";
const LENS_OUTER_HTML: &str = "<!doctype html><html><body><p>Searching with Google Lens...</p><iframe sandbox=\"allow-forms allow-scripts allow-top-navigation\" src=\"/post\" style=\"display:none\"></iframe></body></html>";

pub(crate) fn lens_upload_endpoint(base: &str, query: &str) -> String {
    let query = query.trim();
    if query.is_empty() {
        base.to_owned()
    } else {
        let separator = if base.contains('?') { '&' } else { '?' };
        format!("{base}{separator}q={}", url_encode(query))
    }
}

fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
        let b2 = chunk.get(2).copied().unwrap_or(0) as usize;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[(triple >> 18) & 0x3f] as char);
        out.push(TABLE[(triple >> 12) & 0x3f] as char);
        out.push(if chunk.len() > 1 {
            TABLE[(triple >> 6) & 0x3f] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[triple & 0x3f] as char
        } else {
            '='
        });
    }
    out
}

fn lens_post_html(endpoint: &str, image: &[u8]) -> String {
    let action = serde_json::to_string(endpoint).unwrap_or_else(|_| "\"\"".into());
    let encoded = base64_encode(image);
    format!(
        "<!doctype html><html><body><script>\
        const s=atob(\"{encoded}\");\
        const bytes=new Uint8Array(s.length);\
        for(let i=0;i<s.length;i++)bytes[i]=s.charCodeAt(i);\
        const dt=new DataTransfer();\
        dt.items.add(new File([bytes],\"image.png\",{{type:\"image/png\"}}));\
        const form=document.createElement(\"form\");\
        form.method=\"POST\";\
        form.target=\"_top\";\
        form.action={action};\
        form.enctype=\"multipart/form-data\";\
        const input=document.createElement(\"input\");\
        input.type=\"file\";\
        input.name=\"encoded_image\";\
        input.files=dt.files;\
        form.appendChild(input);\
        document.body.appendChild(form);\
        form.submit();\
        </script></body></html>"
    )
}

fn write_http_response(
    stream: &mut TcpStream,
    status: &str,
    content_type: Option<&str>,
    body: &[u8],
) {
    let mut head = format!(
        "HTTP/1.1 {status}\r\nCache-Control: no-store\r\nContent-Length: {}\r\nConnection: close\r\n",
        body.len()
    );
    if let Some(content_type) = content_type {
        head.push_str(&format!("Content-Type: {content_type}\r\n"));
    }
    head.push_str("\r\n");
    let _ = stream.write_all(head.as_bytes());
    if !body.is_empty() {
        let _ = stream.write_all(body);
    }
    let _ = stream.flush();
}

// Submit the multipart POST from a sandboxed iframe inside the user's browser so Lens
// receives the browser's session cookies and User-Agent with `Origin: null` (non-null localhost
// Origins get 403, and external HTTP uploads lose the browser's Google session cookies).
pub(crate) fn start_lens_handoff(image: Vec<u8>, query: &str) -> Result<String, String> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|error| format!("Couldn't search image: {error}"))?;
    let port = listener
        .local_addr()
        .map_err(|error| format!("Couldn't search image: {error}"))?
        .port();
    listener
        .set_nonblocking(true)
        .map_err(|error| format!("Couldn't search image: {error}"))?;
    let endpoint = lens_upload_endpoint(LENS_UPLOAD_URL, query);
    let post_html = lens_post_html(&endpoint, &image).into_bytes();
    std::thread::spawn(move || {
        let mut deadline = Instant::now() + Duration::from_secs(15);
        while Instant::now() < deadline {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let _ = stream.set_nonblocking(false);
                    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
                    let mut buf = [0u8; 2048];
                    let count = stream.read(&mut buf).unwrap_or(0);
                    let path = std::str::from_utf8(&buf[..count])
                        .ok()
                        .and_then(|text| text.lines().next())
                        .and_then(|line| line.split_whitespace().nth(1))
                        .unwrap_or("/");
                    if path == "/" || path.starts_with("/?") {
                        write_http_response(
                            &mut stream,
                            "200 OK",
                            Some("text/html; charset=utf-8"),
                            LENS_OUTER_HTML.as_bytes(),
                        );
                    } else if path == "/post" {
                        write_http_response(
                            &mut stream,
                            "200 OK",
                            Some("text/html; charset=utf-8"),
                            &post_html,
                        );
                        deadline = (Instant::now() + Duration::from_secs(1)).min(deadline);
                    } else {
                        write_http_response(&mut stream, "204 No Content", None, &[]);
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(15));
                }
                Err(_) => break,
            }
        }
    });
    Ok(format!("http://127.0.0.1:{port}/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn google_lens_handoff_serves_sandboxed_form_with_query_and_base64_image() {
        assert_eq!(
            lens_upload_endpoint("https://lens.google.com/v3/upload?hl=en", "   "),
            "https://lens.google.com/v3/upload?hl=en"
        );
        assert_eq!(
            lens_upload_endpoint("https://lens.google.com/v3/upload?hl=en", "red shoe & box"),
            "https://lens.google.com/v3/upload?hl=en&q=red%20shoe%20%26%20box"
        );
        assert_eq!(base64_encode(b"PNGDATA"), "UE5HREFUQQ==");

        let url = start_lens_handoff(b"PNGDATA".to_vec(), "red shoe").unwrap();
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .build()
            .into();

        let outer = agent
            .get(&url)
            .call()
            .unwrap()
            .into_body()
            .read_to_string()
            .unwrap();
        assert!(outer.contains("sandbox=\"allow-forms allow-scripts allow-top-navigation\""));
        assert!(outer.contains("src=\"/post\""));

        let favicon = agent.get(&format!("{url}favicon.ico")).call().unwrap();
        assert_eq!(favicon.status().as_u16(), 204);

        let post_page = agent
            .get(&format!("{url}post"))
            .call()
            .unwrap()
            .into_body()
            .read_to_string()
            .unwrap();
        assert!(post_page.contains("https://lens.google.com/v3/upload?hl=en&q=red%20shoe"));
        assert!(post_page.contains("UE5HREFUQQ=="));
        assert!(post_page.contains("encoded_image"));
    }
}
