use super::*;

use base64::engine::general_purpose::STANDARD;

fn encode(data: &[u8]) -> String {
    STANDARD.encode(data)
}

#[test]
fn filenames_are_reduced_to_a_safe_basename() {
    assert_eq!(safe_name("notes.md").unwrap(), "notes.md");
    assert_eq!(safe_name("../../etc/passwd").unwrap(), "passwd");
    assert_eq!(safe_name("..\\..\\windows\\system32").unwrap(), "system32");
    assert_eq!(safe_name("").unwrap(), "input");
    assert_eq!(safe_name(".").unwrap(), "input");
    assert_eq!(safe_name("..").unwrap(), "input");
    assert_eq!(safe_name("a/b/").unwrap(), "input");
}

#[test]
fn filenames_with_nul_bytes_are_rejected() {
    assert!(safe_name("bad\0name.txt").is_err());
    assert!(safe_name("\0").is_err());
}

#[test]
fn temp_paths_cannot_escape_their_directory() {
    let dir = tempfile::tempdir().unwrap();
    assert!(tmp_path(dir.path(), "file.txt").is_ok());
    assert!(tmp_path(dir.path(), "sub/file.txt").is_err());
    assert!(tmp_path(dir.path(), "../escape").is_err());
}

#[test]
fn the_envelope_decoder_validates_its_fields() {
    let good = json!({"file": encode(b"hello"), "name": "a.txt"});
    let (data, name) = decode_input(&good).unwrap();
    assert_eq!(data, b"hello");
    assert_eq!(name, "a.txt");

    assert!(decode_input(&json!({}))
        .unwrap_err()
        .contains("missing string field"));
    assert!(decode_input(&json!({"file": 42}))
        .unwrap_err()
        .contains("missing string field"));
    assert!(decode_input(&json!({"file": encode(b"x"), "name": 7}))
        .unwrap_err()
        .contains("'name' must be a string"));
    assert!(decode_input(&json!({"file": "not!base64!"}))
        .unwrap_err()
        .contains("not valid base64"));
}

#[test]
fn inspect_routes_text_and_reports_suspicion() {
    let payload = handle_inspect("clean prose".as_bytes(), "a.txt").unwrap();
    assert_eq!(payload["ok"], true);
    assert_eq!(payload["kind"], "text");
    assert_eq!(payload["suspicious"], false);

    let dirty = handle_inspect("hidden\u{200b}mark".as_bytes(), "a.txt").unwrap();
    assert_eq!(dirty["suspicious"], true);
    assert_eq!(dirty["report"]["suspicious_total"], 1);
}

#[test]
fn inspect_routes_containers_by_the_supplied_name() {
    let payload = handle_inspect(b"---\ngenerator: Claude\n---\nBody\n", "post.md").unwrap();
    assert_eq!(payload["kind"], "container");
    assert_eq!(payload["suspicious"], true);
    assert_eq!(payload["report"]["format"], "markdown");
}

#[test]
fn inspect_refuses_a_binary_container_routed_as_text() {
    // A .txt name over PDF bytes: the extension says text, the sniff refuses.
    let error = handle_inspect(b"%PDF-1.7 body", "a.txt").unwrap_err();
    match error {
        HandlerError::BadRequest(message) => {
            assert!(message.contains("look like a binary container"))
        }
        HandlerError::Internal(message) => panic!("expected a 400, got {message}"),
    }
}

#[test]
fn clean_returns_the_cleaned_bytes_and_a_report() {
    let payload = handle_clean(
        "keep\u{200b}this".as_bytes(),
        "a.txt",
        &json!({"file": "", "options": {}}),
    )
    .unwrap();
    assert_eq!(payload["ok"], true);
    assert_eq!(payload["kind"], "text");
    let cleaned = STANDARD
        .decode(payload["cleaned"].as_str().unwrap())
        .unwrap();
    assert_eq!(cleaned, b"keepthis");
    assert_eq!(payload["report"]["stats"]["removed_count"], 1);
}

#[test]
fn clean_honours_the_option_allowlist() {
    let error = handle_clean(b"x", "a.txt", &json!({"options": {"nope": true}})).unwrap_err();
    match error {
        HandlerError::BadRequest(message) => assert_eq!(message, "unknown option: nope"),
        HandlerError::Internal(message) => panic!("expected a 400, got {message}"),
    }

    let error = handle_clean(b"x", "a.txt", &json!({"options": []})).unwrap_err();
    match error {
        HandlerError::BadRequest(message) => assert_eq!(message, "'options' must be an object"),
        HandlerError::Internal(message) => panic!("expected a 400, got {message}"),
    }
}

#[test]
fn clean_passes_text_options_through() {
    let payload = handle_clean(
        "ﬁle".as_bytes(),
        "a.txt",
        &json!({"options": {"nfkc": true}}),
    )
    .unwrap();
    let cleaned = STANDARD
        .decode(payload["cleaned"].as_str().unwrap())
        .unwrap();
    assert_eq!(String::from_utf8(cleaned).unwrap(), "file");
}

#[test]
fn clean_validates_the_pixel_backend_name() {
    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    png.extend_from_slice(&prose_sanitiser::image::png::build_chunk(
        b"IHDR",
        &[0, 0, 0, 1, 0, 0, 0, 1, 8, 0, 0, 0, 0],
    ));
    png.extend_from_slice(&prose_sanitiser::image::png::build_chunk(b"IEND", b""));

    let error = handle_clean(
        &png,
        "a.png",
        &json!({"options": {"remove_pixel": "magic"}}),
    )
    .unwrap_err();
    match error {
        HandlerError::BadRequest(message) => {
            assert_eq!(message, "remove_pixel must be one of: ctrlregen, diffusion")
        }
        HandlerError::Internal(message) => panic!("expected a 400, got {message}"),
    }
}

#[test]
fn clean_strips_container_metadata_and_hides_the_temp_paths() {
    let payload = handle_clean(
        b"---\ngenerator: Claude\ntitle: Hills\n---\nBody\n",
        "post.md",
        &json!({"options": {}}),
    )
    .unwrap();
    assert_eq!(payload["kind"], "container");
    let cleaned = STANDARD
        .decode(payload["cleaned"].as_str().unwrap())
        .unwrap();
    let text = String::from_utf8(cleaned).unwrap();
    assert!(!text.contains("Claude"));
    assert!(text.contains("title: Hills"));
    // The request temp directory must never leak into the response.
    assert!(payload["report"].get("input").is_none());
    assert!(payload["report"].get("output").is_none());
    assert_eq!(payload["report"]["kind"], "container");

    // Key order must survive the temp-path removal: `preserve_order`'s
    // `remove` is a swap-remove, which would drag `meta` and `post_findings`
    // forward and diverge from the Python's `dict.pop`.
    let keys: Vec<&str> = payload["report"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        keys,
        vec![
            "kind",
            "format",
            "actions",
            "bytes_in",
            "bytes_out",
            "still_has_c2pa",
            "still_has_ai_metadata",
            "post_findings",
            "meta",
        ]
    );
}

#[test]
fn capabilities_reports_every_backend_slot() {
    let payload = capabilities();
    assert!(payload["tools"]["exiftool"].is_boolean());
    assert!(payload["tools"]["qpdf"].is_boolean());
    assert!(payload["pixel_backends"]["ctrlregen"].is_boolean());
    assert!(payload["pixel_backends"]["diffusion"].is_boolean());
    assert!(payload["scorers"]["synthid"].is_boolean());
    assert!(payload["harnesses"]["markllm"].is_boolean());
}

#[test]
fn the_body_cap_leaves_headroom_for_base64_inflation() {
    // Base64 inflates by 4/3, so the envelope cap must exceed the input cap.
    assert!(max_body_bytes() as u64 > max_input_bytes());
}

fn bearer(value: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert("authorization", value.parse().unwrap());
    headers
}

#[test]
fn the_bearer_key_is_compared_exactly() {
    let state = ServerState {
        api_key: Some("s3cret".to_string()),
        ..ServerState::default()
    };
    assert!(authorised(&state, &bearer("Bearer s3cret")));
    assert!(!authorised(&state, &bearer("Bearer s3cre")));
    assert!(!authorised(&state, &bearer("Bearer s3cret ")));
    assert!(!authorised(&state, &bearer("Bearer s3creT")));
    assert!(!authorised(&state, &bearer("bearer s3cret")));
    assert!(!authorised(&state, &bearer("s3cret")));
    assert!(!authorised(&state, &HeaderMap::new()));
    // A non-UTF-8 header value is a mismatch, not a panic.
    let mut odd = HeaderMap::new();
    odd.insert(
        "authorization",
        axum::http::HeaderValue::from_bytes(b"Bearer s3cr\xe9t").unwrap(),
    );
    assert!(!authorised(&state, &odd));
}

#[test]
fn no_key_configured_means_every_request_is_authorised() {
    // Documented in crates/server/README.md: an unset key means no auth, so the
    // service is meant for a loopback bind. Making the key mandatory is a
    // behaviour change that has not been decided (COV-A-F7).
    let state = ServerState::default();
    assert!(authorised(&state, &HeaderMap::new()));
}

// ---------------------------------------------------------------------------
// Browser-origin hardening: Content-Type and Host validation.
// ---------------------------------------------------------------------------

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};

/// Serve `app` on an ephemeral loopback port on a background thread. `make`
/// receives the bound address so it can build the state the binary would.
fn serve(make: impl FnOnce(SocketAddr) -> ServerState + Send + 'static) -> SocketAddr {
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async move {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            sender.send(addr).unwrap();
            axum::serve(listener, app(make(addr))).await.unwrap();
        });
    });
    receiver.recv().unwrap()
}

/// Send one raw HTTP/1.1 request and return `(status, body)`.
fn send(addr: SocketAddr, head: &[&str], body: &str) -> (u16, String) {
    let mut request = head.join("\r\n");
    request.push_str(&format!(
        "\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    ));
    let mut stream = TcpStream::connect(addr).unwrap();
    stream.write_all(request.as_bytes()).unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    let status = response
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or_else(|| panic!("no status in {response:?}"));
    let body = response
        .split_once("\r\n\r\n")
        .map(|(_, body)| body.to_string())
        .unwrap_or_default();
    (status, body)
}

fn inspect_body() -> String {
    format!(r#"{{"file": "{}", "name": "a.txt"}}"#, encode(b"hello"))
}

fn post_inspect(addr: SocketAddr, host: &str, content_type: Option<&str>) -> (u16, String) {
    let mut head = vec![
        "POST /inspect HTTP/1.1".to_string(),
        format!("Host: {host}"),
    ];
    if let Some(content_type) = content_type {
        head.push(format!("Content-Type: {content_type}"));
    }
    let head: Vec<&str> = head.iter().map(String::as_str).collect();
    send(addr, &head, &inspect_body())
}

#[test]
fn json_routes_require_a_json_content_type() {
    let addr = serve(|bound| ServerState {
        api_key: None,
        bind: Some(bound),
    });
    let host = format!("127.0.0.1:{}", addr.port());
    // The cross-origin "simple request" content types a page can send without a
    // preflight, and a missing header.
    for bad in [
        Some("text/plain"),
        Some("text/plain;charset=UTF-8"),
        Some("application/x-www-form-urlencoded"),
        Some("multipart/form-data; boundary=x"),
        Some("application/jsonp"),
        Some("application/json-seq"),
        None,
    ] {
        let (status, body) = post_inspect(addr, &host, bad);
        assert_eq!(status, 415, "{bad:?} -> {body}");
        assert!(body.contains("application/json"), "{body}");
    }
    for good in [
        "application/json",
        "application/json; charset=utf-8",
        "Application/JSON",
        "application/json ;charset=UTF-8",
    ] {
        let (status, body) = post_inspect(addr, &host, Some(good));
        assert_eq!(status, 200, "{good} -> {body}");
    }
    // /clean is gated the same way.
    let head = [
        "POST /clean HTTP/1.1",
        &format!("Host: {host}"),
        "Content-Type: text/plain",
    ];
    assert_eq!(send(addr, &head, &inspect_body()).0, 415);
}

#[test]
fn a_loopback_bind_refuses_a_foreign_host_header() {
    let addr = serve(|bound| ServerState {
        api_key: None,
        bind: Some(bound),
    });
    let port = addr.port();
    // DNS rebinding: the page's origin is attacker.example, resolving to us.
    for bad in [
        "attacker.example".to_string(),
        format!("attacker.example:{port}"),
        format!("localhost.attacker.example:{port}"),
        format!("127.0.0.1.attacker.example:{port}"),
        format!("127.0.0.1:{}", port.wrapping_add(1)),
        format!("localhost:{}", port.wrapping_add(1)),
        "10.0.0.5:80".to_string(),
        String::new(),
    ] {
        let (status, body) = post_inspect(addr, &bad, Some("application/json"));
        assert_eq!(status, 421, "Host {bad:?} -> {body}");
        // Reads are refused too: rebinding exists to read the replies.
        let head = ["GET /capabilities HTTP/1.1", &format!("Host: {bad}")];
        assert_eq!(send(addr, &head, "").0, 421, "GET with Host {bad:?}");
    }
    // No Host header at all (HTTP/1.0 style) is refused.
    let (status, _) = send(addr, &["GET /health HTTP/1.0"], "");
    assert_eq!(status, 421);
}

#[test]
fn a_loopback_bind_serves_its_own_names() {
    let addr = serve(|bound| ServerState {
        api_key: None,
        bind: Some(bound),
    });
    let port = addr.port();
    for good in [
        format!("127.0.0.1:{port}"),
        format!("localhost:{port}"),
        format!("LOCALHOST:{port}"),
        format!("localhost.:{port}"),
        "localhost".to_string(),
    ] {
        let (status, body) = post_inspect(addr, &good, Some("application/json"));
        assert_eq!(status, 200, "Host {good:?} -> {body}");
        let head = ["GET /health HTTP/1.1", &format!("Host: {good}")];
        assert_eq!(send(addr, &head, "").0, 200, "GET with Host {good:?}");
    }
    // A legitimate request returns the real report.
    let (status, body) = post_inspect(addr, &format!("localhost:{port}"), Some("application/json"));
    assert_eq!(status, 200);
    assert!(body.contains("\"ok\": true"), "{body}");
}

fn host_header(value: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert("host", value.parse().unwrap());
    headers
}

#[test]
fn host_validation_follows_the_bind_address() {
    let loopback = |ip: &str| ServerState {
        api_key: None,
        bind: Some(format!("{ip}:8765").parse().unwrap()),
    };
    // IPv6 loopback bind: its own literal and the loopback names are accepted.
    let v6 = loopback("[::1]");
    assert!(host_allowed(&v6, &host_header("[::1]:8765")));
    assert!(host_allowed(&v6, &host_header("localhost:8765")));
    assert!(!host_allowed(&v6, &host_header("[::2]:8765")));
    assert!(!host_allowed(&v6, &host_header("[::1")));
    assert!(!host_allowed(&v6, &host_header("[::1]:notaport")));
    // Any 127.0.0.0/8 bind accepts its own literal.
    let other = loopback("127.0.0.2");
    assert!(host_allowed(&other, &host_header("127.0.0.2:8765")));
    assert!(host_allowed(&other, &host_header("127.0.0.1:8765")));

    // A non-loopback bind is a deliberate network deployment: its names are not
    // knowable here, so the check is off (documented).
    let lan = ServerState {
        api_key: None,
        bind: Some("0.0.0.0:8765".parse().unwrap()),
    };
    assert!(host_allowed(&lan, &host_header("whatever.example")));
    // A bearer key is the control when one is set: a rebound page cannot send it.
    let keyed = ServerState {
        api_key: Some("k".into()),
        bind: Some("127.0.0.1:8765".parse().unwrap()),
    };
    assert!(host_allowed(&keyed, &host_header("proxy.example")));
    // An unknown bind is treated as loopback-only, with any port.
    let unknown = ServerState::default();
    assert!(host_allowed(&unknown, &host_header("localhost:1234")));
    assert!(!host_allowed(&unknown, &host_header("evil.example")));
}

#[test]
fn a_keyed_server_still_requires_the_key_and_json() {
    let addr = serve(|bound| ServerState {
        api_key: Some("s3cret".into()),
        bind: Some(bound),
    });
    let host = format!("127.0.0.1:{}", addr.port());
    let (status, _) = post_inspect(addr, &host, Some("application/json"));
    assert_eq!(status, 401);
    let head = [
        "POST /inspect HTTP/1.1",
        &format!("Host: {host}"),
        "Content-Type: application/json",
        "Authorization: Bearer s3cret",
    ];
    assert_eq!(send(addr, &head, &inspect_body()).0, 200);
    let head = [
        "POST /inspect HTTP/1.1",
        &format!("Host: {host}"),
        "Content-Type: text/plain",
        "Authorization: Bearer s3cret",
    ];
    assert_eq!(send(addr, &head, &inspect_body()).0, 415);
}
