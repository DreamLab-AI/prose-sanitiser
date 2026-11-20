//! HTTP service exposing the cleaning pipeline.
//!
//! The agent skill and any web app can call this instead of running the CLIs
//! locally. Hardening mirrors the CLIs: input size caps, the binary-as-text
//! guard, atomic writes, a loopback-only bind by default and an optional bearer
//! key. Intended for a trusted network; put it behind a reverse proxy if it is
//! reachable from untrusted clients.
//!
//! # Browser-origin hardening
//!
//! A web page open in a browser on the same machine can reach a loopback
//! service, so two checks run whether or not a key is set:
//!
//! - **`Content-Type: application/json` is required** on `POST /inspect` and
//!   `POST /clean`; anything else, or no header, is `415`. A page can send
//!   `text/plain` (or form types) cross-origin *without* a CORS preflight, and
//!   the body would otherwise be parsed as JSON regardless. The service sends no
//!   CORS headers and answers no preflight, so a real `application/json`
//!   request from another origin is blocked by the browser before it is sent.
//! - **The `Host` header is validated** against the bound address when the
//!   service is bound to loopback and no bearer key is set; a mismatch is
//!   `421 Misdirected Request`, on every route. DNS rebinding makes an
//!   attacker's hostname resolve to `127.0.0.1`, which makes the page
//!   same-origin and lets it read replies; it cannot make the browser send a
//!   different `Host`. Accepted: `localhost`, `127.0.0.1`, `[::1]` and the bound
//!   IP literal, each bare or with the bound port. Not enforced when a key is
//!   set (a rebound page cannot present it, and a reverse proxy may forward its
//!   own `Host`) or when bound to a non-loopback address (the names that reach a
//!   network deployment are not knowable from here).
//!
//! Routes:
//! - `GET  /health`       -> `{"ok": true, "version": ...}`
//! - `GET  /capabilities` -> which optional tools / pixel backends are present
//! - `GET  /openapi.json` -> the generated OpenAPI 3.0.3 spec
//! - `GET  /rules`        -> the stylistic rule table: ids, tiers, dates, sources and the ruleset changelog
//! - `POST /inspect`      -> `{"file": <base64>, "name": "x.png"}` -> findings
//! - `POST /clean`        -> `{"file": ..., "options": {...}}` -> cleaned bytes

#![deny(missing_docs)]

pub mod openapi;

use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use axum::body::Bytes;
use axum::extract::Request;
use axum::extract::{DefaultBodyLimit, State};
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::Router;
use base64::Engine;
use serde_json::{json, Map, Value};
use subtle::ConstantTimeEq;

use prose_sanitiser::common::io::max_input_bytes;
use prose_sanitiser::common::surrogate;
use prose_sanitiser::common::{env_dir, env_nonempty, looks_binary, to_pretty_json, which};
use prose_sanitiser::container::{clean_container, inspect_container};
use prose_sanitiser::dispatch::{classify_bytes, Kind};
use prose_sanitiser::image::{clean_image, inspect_image, CleanImageOptions, PixelRemover};
use prose_sanitiser::slop::rules::{rule_meta, CHANGELOG, RULESET_REVIEWED, RULESET_VERSION};
use prose_sanitiser::text::{clean_text, inspect_text, CleanOptions};
use prose_sanitiser_core::{classify_finding_confidence, ConfidenceTier};

/// The advertised version.
pub fn version() -> &'static str {
    static VALUE: OnceLock<String> = OnceLock::new();
    VALUE.get_or_init(|| env_nonempty("WATERMARKS_SERVER_VERSION").unwrap_or_else(|| "dev".into()))
}

/// A newtype so the OpenAPI module can read the version without a cycle.
pub struct Version;

impl Version {
    /// The advertised version string, identical to [`version()`].
    pub fn as_str(&self) -> &'static str {
        version()
    }
}

/// The singleton [`Version`] handle the OpenAPI module reads.
pub const VERSION: Version = Version;

/// The clean options the service accepts, and whether each is a boolean.
pub const ALLOWED_CLEAN_OPTIONS: &[(&str, bool)] = &[
    ("nfkc", true),
    ("aggressive_homoglyphs", true),
    ("keep_non_ai_metadata", true),
    ("also_layer_a_text", true),
    ("remove_pixel", false),
    ("strip_all_metadata", true),
];

/// Body cap for the JSON envelope. Base64 inflates by 4/3, so the decoded file
/// stays well under the input cap for the same envelope cap.
pub fn max_body_bytes() -> usize {
    let cap = max_input_bytes() as usize;
    cap + (cap >> 1)
}

/// Runtime state: the bearer token, when one is configured, and the bound address.
#[derive(Clone, Default)]
pub struct ServerState {
    /// The bearer token every request must present, or `None` when the
    /// service is running unauthenticated on a loopback bind.
    pub api_key: Option<String>,
    /// The address the listener is actually bound to (`local_addr()`, not the
    /// requested name). Drives `Host` validation; `None` is treated as a
    /// loopback bind on an unknown port, the strictest reading.
    pub bind: Option<SocketAddr>,
}

/// The rule table behind every stylistic finding: version, dates and tiers.
///
/// A client that cannot see which ruleset produced a finding cannot tell a
/// current report from one made by a build whose lexical markers went stale two
/// years ago. Every response carries the version; this is the detail behind it.
pub fn ruleset() -> Value {
    let mut tiers = Map::new();
    for tier in [
        ConfidenceTier::CertainMechanical,
        ConfidenceTier::HighConfidenceStylistic,
        ConfidenceTier::LowConfidenceJudgement,
    ] {
        let count = rule_meta()
            .iter()
            .filter(|meta| meta.confidence == tier)
            .count();
        tiers.insert(tier.as_str().to_string(), json!(count));
    }
    json!({
        "version": RULESET_VERSION,
        "reviewed": RULESET_REVIEWED,
        "rules": rule_meta().len(),
        "tiers": Value::Object(tiers),
        "note": "Stylistic rules are population-level signals, never forensic. No slop rule is certain-mechanical, so none is ever auto-fixed.",
    })
}

/// Every rule the stylistic layer can emit, with its tier, dates and sources.
pub fn rules_table() -> Value {
    json!({
        "ok": true,
        "ruleset_version": RULESET_VERSION,
        "rules": rule_meta()
            .iter()
            .map(|meta| json!({
                "id": meta.id,
                "name": meta.name,
                "description": meta.description,
                "severity": meta.severity.as_str(),
                "confidence": meta.confidence.as_str(),
                "auto_fixable": meta.confidence.auto_fixable(),
                "since": meta.since,
                "reviewed": meta.reviewed,
                "sources": meta.sources,
            }))
            .collect::<Vec<_>>(),
        "changelog": CHANGELOG
            .iter()
            .map(|entry| json!({
                "version": entry.version,
                "date": entry.date,
                "notes": entry.notes,
            }))
            .collect::<Vec<_>>(),
    })
}

/// Which optional tools and heavy backends are present.
pub fn capabilities() -> Value {
    json!({
        "version": version(),
        "tools": {
            "c2patool": which("c2patool").is_some(),
            "exiftool": which("exiftool").is_some(),
            "qpdf": which("qpdf").is_some(),
        },
        "pixel_backends": {
            "ctrlregen": env_dir("NOAI_WATERMARK_DIR").is_some(),
            "diffusion": env_dir("MARKDIFFUSION_DIR").is_some(),
        },
        "scorers": {"synthid": env_dir("REVERSE_SYNTHID_DIR").is_some()},
        "harnesses": {"markllm": env_dir("MARKLLM_DIR").is_some()},
        "ruleset": ruleset(),
    })
}

/// Every response is pretty-printed JSON with a no-store cache header, as the
/// Python's `_respond` produced.
fn respond(status: StatusCode, payload: Value) -> Response {
    let body = to_pretty_json(&payload);
    (
        status,
        [
            ("content-type", "application/json; charset=utf-8"),
            ("cache-control", "no-store"),
        ],
        body,
    )
        .into_response()
}

fn error(status: StatusCode, message: &str) -> Response {
    respond(status, json!({"ok": false, "error": message}))
}

fn authorised(state: &ServerState, headers: &HeaderMap) -> bool {
    let Some(key) = &state.api_key else {
        return true;
    };
    let expected = format!("Bearer {key}");
    // Constant-time comparison (COV-A-F7): `==` on strings stops at the first
    // differing byte, which leaks how much of a guessed key was right. `subtle`
    // still returns early when the lengths differ, so the length of the key is
    // not hidden; its value is.
    headers
        .get("authorization")
        .map(|value| bool::from(value.as_bytes().ct_eq(expected.as_bytes())))
        .unwrap_or(false)
}

/// Split a `Host` header into its name (lowercased, brackets kept for IPv6, no
/// trailing dot) and optional port. `None` when it is malformed.
fn split_host(value: &str) -> Option<(String, Option<u16>)> {
    let (name, port) = if value.starts_with('[') {
        let end = value.find(']')?;
        let rest = &value[end + 1..];
        let port = match rest {
            "" => None,
            _ => Some(rest.strip_prefix(':')?),
        };
        (&value[..=end], port)
    } else {
        match value.rsplit_once(':') {
            Some((name, port)) => (name, Some(port)),
            None => (value, None),
        }
    };
    let port = match port {
        None => None,
        Some(text) => Some(text.parse::<u16>().ok()?),
    };
    let name = name.trim_end_matches('.').to_ascii_lowercase();
    if name.is_empty() {
        return None;
    }
    Some((name, port))
}

/// Whether the request's `Host` header is acceptable for this server.
///
/// Only enforced for a loopback bind with no bearer key; see the crate docs.
fn host_allowed(state: &ServerState, headers: &HeaderMap) -> bool {
    if state.api_key.is_some() {
        return true;
    }
    let bound_ip = state.bind.map(|addr| addr.ip());
    if matches!(bound_ip, Some(ip) if !ip.is_loopback()) {
        return true;
    }
    let Some((name, port)) = headers
        .get("host")
        .and_then(|value| value.to_str().ok())
        .and_then(split_host)
    else {
        return false;
    };
    if let (Some(port), Some(bound)) = (port, state.bind) {
        if port != bound.port() {
            return false;
        }
    }
    let literal = |ip: IpAddr| match ip {
        IpAddr::V4(v4) => v4.to_string(),
        IpAddr::V6(v6) => format!("[{v6}]"),
    };
    matches!(name.as_str(), "localhost" | "127.0.0.1" | "[::1]")
        || bound_ip.is_some_and(|ip| literal(ip) == name)
}

/// Reject a request whose `Host` is not one this server answers to (`421`).
async fn host_guard(
    State(state): State<Arc<ServerState>>,
    request: Request,
    next: Next,
) -> Response {
    if !host_allowed(&state, request.headers()) {
        return error(StatusCode::MISDIRECTED_REQUEST, "unrecognised Host header");
    }
    next.run(request).await
}

/// Whether the request declares a JSON body. Parameters such as `charset` are
/// allowed; the media type is compared case-insensitively and exactly.
fn is_json_content_type(headers: &HeaderMap) -> bool {
    headers
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .is_some_and(|media| media.trim().eq_ignore_ascii_case("application/json"))
}

/// Reduce a client-supplied filename to a bare basename safe for temp use.
///
/// A name like `../../x` would otherwise let a write escape the request temp
/// dir. Windows separators are folded too, and `.`/`..`/empty fall back to a
/// neutral name.
pub fn safe_name(name: &str) -> Result<String, String> {
    // NUL bytes cannot appear in filesystem paths; reject them as a client error
    // rather than letting the write fail with an internal error.
    if name.contains('\0') {
        return Err("filename contains NUL".to_string());
    }
    let normalised = name.replace('\\', "/");
    let base = normalised.rsplit('/').next().unwrap_or("");
    if base.is_empty() || base == "." || base == ".." {
        return Ok("input".to_string());
    }
    Ok(base.to_string())
}

/// Join `part` under `dir`, refusing anything that escapes it.
///
/// Defence in depth: even if a caller slips a separator through, the write can
/// never land outside the request temp dir.
pub fn tmp_path(dir: &Path, part: &str) -> Result<PathBuf, String> {
    let path = dir.join(part);
    if path.parent() != Some(dir) {
        return Err("unsafe filename".to_string());
    }
    Ok(path)
}

/// Decode the `{file, name}` envelope.
fn decode_input(body: &Value) -> Result<(Vec<u8>, String), String> {
    let raw = body
        .get("file")
        .and_then(Value::as_str)
        .ok_or("missing string field 'file' (base64-encoded bytes)")?;
    let name = match body.get("name") {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(value)) => value.clone(),
        Some(_) => return Err("'name' must be a string".to_string()),
    };
    let data = base64::engine::general_purpose::STANDARD
        .decode(raw)
        .map_err(|_| "'file' is not valid base64".to_string())?;
    // Check the DECODED length against the input cap. The base64 envelope is
    // larger, but the per-format cap applies to the file itself.
    let cap = max_input_bytes() as usize;
    if data.len() > cap {
        return Err(format!(
            "decoded file is {} bytes, exceeding the {} byte limit",
            data.len(),
            cap
        ));
    }
    Ok((data, safe_name(&name)?))
}

fn suffix_of(name: &str) -> Option<String> {
    Path::new(name)
        .extension()
        .map(|ext| ext.to_string_lossy().into_owned())
}

async fn health(State(state): State<Arc<ServerState>>, headers: HeaderMap) -> Response {
    if !authorised(&state, &headers) {
        return error(StatusCode::UNAUTHORIZED, "unauthorized");
    }
    respond(
        StatusCode::OK,
        json!({
            "ok": true,
            "version": version(),
            "ruleset_version": RULESET_VERSION,
        }),
    )
}

async fn capabilities_route(State(state): State<Arc<ServerState>>, headers: HeaderMap) -> Response {
    if !authorised(&state, &headers) {
        return error(StatusCode::UNAUTHORIZED, "unauthorized");
    }
    let mut payload = Map::new();
    payload.insert("ok".into(), json!(true));
    if let Some(map) = capabilities().as_object() {
        for (key, value) in map {
            payload.insert(key.clone(), value.clone());
        }
    }
    respond(StatusCode::OK, Value::Object(payload))
}

async fn rules_route(State(state): State<Arc<ServerState>>, headers: HeaderMap) -> Response {
    if !authorised(&state, &headers) {
        return error(StatusCode::UNAUTHORIZED, "unauthorized");
    }
    respond(StatusCode::OK, rules_table())
}

async fn openapi_route(State(state): State<Arc<ServerState>>, headers: HeaderMap) -> Response {
    if !authorised(&state, &headers) {
        return error(StatusCode::UNAUTHORIZED, "unauthorized");
    }
    respond(
        StatusCode::OK,
        openapi::openapi_spec(state.api_key.is_some()),
    )
}

/// Parse the request body, mapping an oversize or malformed body to its status.
// The Err variant is a whole axum Response; boxing it would buy nothing
// here, since the caller returns it immediately.
#[allow(clippy::result_large_err)]
fn read_json(headers: &HeaderMap, body: &Bytes) -> Result<Value, Response> {
    // Before anything is parsed: a cross-origin page can send `text/plain`
    // without a preflight, so the body is refused unless it says it is JSON.
    if !is_json_content_type(headers) {
        return Err(error(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Content-Type must be application/json",
        ));
    }
    if body.len() > max_body_bytes() {
        return Err(error(StatusCode::PAYLOAD_TOO_LARGE, "invalid request body"));
    }
    match serde_json::from_slice::<Value>(body) {
        Ok(Value::Object(map)) => Ok(Value::Object(map)),
        _ => Err(error(StatusCode::BAD_REQUEST, "invalid request body")),
    }
}

async fn inspect_route(
    State(state): State<Arc<ServerState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if !authorised(&state, &headers) {
        return error(StatusCode::UNAUTHORIZED, "unauthorized");
    }
    let body = match read_json(&headers, &body) {
        Ok(body) => body,
        Err(response) => return response,
    };
    let (data, name) = match decode_input(&body) {
        Ok(pair) => pair,
        Err(message) => return error(StatusCode::BAD_REQUEST, &message),
    };
    match handle_inspect(&data, &name) {
        Ok(payload) => respond(StatusCode::OK, payload),
        Err(HandlerError::BadRequest(message)) => error(StatusCode::BAD_REQUEST, &message),
        Err(HandlerError::Internal(message)) => {
            eprintln!("error handling /inspect: {message}");
            error(StatusCode::INTERNAL_SERVER_ERROR, "internal error")
        }
    }
}

async fn clean_route(
    State(state): State<Arc<ServerState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if !authorised(&state, &headers) {
        return error(StatusCode::UNAUTHORIZED, "unauthorized");
    }
    let body = match read_json(&headers, &body) {
        Ok(body) => body,
        Err(response) => return response,
    };
    let (data, name) = match decode_input(&body) {
        Ok(pair) => pair,
        Err(message) => return error(StatusCode::BAD_REQUEST, &message),
    };
    match handle_clean(&data, &name, &body) {
        Ok(payload) => respond(StatusCode::OK, payload),
        Err(HandlerError::BadRequest(message)) => error(StatusCode::BAD_REQUEST, &message),
        Err(HandlerError::Internal(message)) => {
            eprintln!("error handling /clean: {message}");
            error(StatusCode::INTERNAL_SERVER_ERROR, "internal error")
        }
    }
}

async fn not_found() -> Response {
    error(StatusCode::NOT_FOUND, "not found")
}

/// A handler failure, split by the status it maps to.
#[derive(Debug)]
pub enum HandlerError {
    /// The caller's fault: malformed envelope, unsupported option or input
    /// over the cap. Maps to `400`, and the message is returned to the caller.
    BadRequest(String),
    /// The service's fault: an I/O or pipeline failure. Maps to `500`.
    Internal(String),
}

/// Inspect bytes, routing by the name's extension then the magic bytes.
pub fn handle_inspect(data: &[u8], name: &str) -> Result<Value, HandlerError> {
    let kind = classify_bytes(data, suffix_of(name).as_deref());
    let dir = tempfile::Builder::new()
        .prefix("wm-inspect-")
        .tempdir()
        .map_err(|error| HandlerError::Internal(error.to_string()))?;
    let path = tmp_path(dir.path(), if name.is_empty() { "input" } else { name })
        .map_err(HandlerError::BadRequest)?;
    std::fs::write(&path, data).map_err(|error| HandlerError::Internal(error.to_string()))?;

    let report = match kind {
        Kind::Text => {
            if looks_binary(data).is_some() {
                return Err(HandlerError::BadRequest(
                    "refusing to inspect bytes that look like a binary container as text".into(),
                ));
            }
            inspect_text(&surrogate::decode(data), false, false).to_json()
        }
        Kind::Image => inspect_image(&path, None)
            .map_err(|error| HandlerError::Internal(error.to_string()))?
            .to_json(),
        Kind::Container => inspect_container(&path)
            .map_err(|error| HandlerError::Internal(error.to_string()))?
            .to_json(),
    };

    let suspicious = report
        .get("suspicious_total")
        .and_then(Value::as_u64)
        .map(|total| total > 0)
        .unwrap_or(false)
        || report
            .get("has_c2pa")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        || report
            .get("has_ai_metadata")
            .and_then(Value::as_bool)
            .unwrap_or(false);

    let tiers = tier_counts(&report);
    Ok(json!({
        "ok": true,
        "kind": kind.as_str(),
        "report": report,
        "suspicious": suspicious,
        // Additive: appended after every long-standing key, so a client that
        // reads the document by key is unaffected and one that diffs it sees
        // only new lines at the end.
        "ruleset_version": RULESET_VERSION,
        "tiers": tiers,
    }))
}

/// Bucket a report's finding strings by confidence tier.
///
/// The media scanners report prose strings rather than typed rules, so the
/// tier comes from the same classifier the inspect binaries print. A parsed
/// provenance structure is mechanical; a note about an unsupported format or a
/// raw byte scan is a judgement call.
fn tier_counts(report: &Value) -> Value {
    let mut mechanical = 0usize;
    let mut judgement = 0usize;
    if let Some(findings) = report.get("findings").and_then(Value::as_array) {
        for finding in findings.iter().filter_map(Value::as_str) {
            match classify_finding_confidence(finding) {
                "confirmed" | "probable" => mechanical += 1,
                _ => judgement += 1,
            }
        }
    }
    // Layer A carriers are deterministic codepoint classifications, so every
    // suspicious character is mechanical.
    if let Some(total) = report.get("suspicious_total").and_then(Value::as_u64) {
        mechanical += total as usize;
    }
    json!({
        ConfidenceTier::CertainMechanical.as_str(): mechanical,
        ConfidenceTier::HighConfidenceStylistic.as_str(): 0,
        ConfidenceTier::LowConfidenceJudgement.as_str(): judgement,
    })
}

/// Clean bytes, returning the cleaned bytes base64-encoded plus a report.
pub fn handle_clean(data: &[u8], name: &str, body: &Value) -> Result<Value, HandlerError> {
    let kind = classify_bytes(data, suffix_of(name).as_deref());
    let options = match body.get("options") {
        None | Some(Value::Null) => Map::new(),
        Some(Value::Object(map)) => map.clone(),
        Some(_) => {
            return Err(HandlerError::BadRequest(
                "'options' must be an object".into(),
            ))
        }
    };
    for key in options.keys() {
        if !ALLOWED_CLEAN_OPTIONS.iter().any(|(name, _)| name == key) {
            return Err(HandlerError::BadRequest(format!("unknown option: {key}")));
        }
    }
    let flag = |key: &str| options.get(key).and_then(Value::as_bool).unwrap_or(false);

    let dir = tempfile::Builder::new()
        .prefix("wm-clean-")
        .tempdir()
        .map_err(|error| HandlerError::Internal(error.to_string()))?;
    let source = tmp_path(dir.path(), if name.is_empty() { "input" } else { name })
        .map_err(HandlerError::BadRequest)?;
    std::fs::write(&source, data).map_err(|error| HandlerError::Internal(error.to_string()))?;

    let (cleaned_bytes, mut report) = match kind {
        Kind::Text => {
            if looks_binary(data).is_some() {
                return Err(HandlerError::BadRequest(
                    "refusing to clean bytes that look like a binary container as text".into(),
                ));
            }
            let units = surrogate::decode(data);
            let (cleaned, stats) = clean_text(
                &units,
                CleanOptions {
                    nfkc: flag("nfkc"),
                    aggressive_homoglyphs: flag("aggressive_homoglyphs"),
                    ..CleanOptions::default()
                },
            );
            let length = cleaned.len();
            (
                surrogate::encode(&cleaned),
                json!({"kind": "text", "stats": stats.to_json(), "length": length}),
            )
        }
        Kind::Image => {
            let dest = dir.path().join("out.png");
            let mut strip_all = !flag("keep_non_ai_metadata");
            if let Some(explicit) = options.get("strip_all_metadata").and_then(Value::as_bool) {
                strip_all = explicit;
            }
            let remove_pixel = match options.get("remove_pixel") {
                None | Some(Value::Null) => None,
                Some(Value::String(value)) => {
                    Some(PixelRemover::parse(value).ok_or_else(|| {
                        HandlerError::BadRequest(
                            "remove_pixel must be one of: ctrlregen, diffusion".into(),
                        )
                    })?)
                }
                Some(_) => {
                    return Err(HandlerError::BadRequest(
                        "remove_pixel must be one of: ctrlregen, diffusion".into(),
                    ))
                }
            };
            let result = clean_image(
                &source,
                &dest,
                &CleanImageOptions {
                    strip_all_metadata: strip_all,
                    remove_pixel,
                    ..CleanImageOptions::default()
                },
            )
            .map_err(HandlerError::Internal)?;
            let bytes =
                std::fs::read(&dest).map_err(|error| HandlerError::Internal(error.to_string()))?;
            (bytes, merge_kind("image", result))
        }
        Kind::Container => {
            let suffix = suffix_of(name)
                .map(|ext| format!(".{ext}"))
                .unwrap_or_default();
            let dest =
                tmp_path(dir.path(), &format!("out{suffix}")).map_err(HandlerError::BadRequest)?;
            let also_layer_a = options
                .get("also_layer_a_text")
                .and_then(Value::as_bool)
                .unwrap_or(true);
            let result =
                clean_container(&source, &dest, also_layer_a).map_err(HandlerError::Internal)?;
            let bytes =
                std::fs::read(&dest).map_err(|error| HandlerError::Internal(error.to_string()))?;
            (bytes, merge_kind("container", result))
        }
    };

    // The temp paths are an implementation detail, never a response field.
    // `shift_remove`, not `remove`: under `preserve_order` the latter is a
    // swap-remove, which would drag the last two keys forward and reorder the
    // report relative to the Python's `dict.pop`.
    if let Some(map) = report.as_object_mut() {
        map.shift_remove("input");
        map.shift_remove("output");
    }

    Ok(json!({
        "ok": true,
        "kind": kind.as_str(),
        "cleaned": base64::engine::general_purpose::STANDARD.encode(&cleaned_bytes),
        "report": report,
        "ruleset_version": RULESET_VERSION,
    }))
}

/// `{"kind": kind, **result}` — the Python's dict-splat ordering.
fn merge_kind(kind: &str, result: Value) -> Value {
    let mut map = Map::new();
    map.insert("kind".into(), json!(kind));
    if let Some(fields) = result.as_object() {
        for (key, value) in fields {
            map.insert(key.clone(), value.clone());
        }
    }
    Value::Object(map)
}

/// Build the router.
pub fn app(state: ServerState) -> Router {
    let state = Arc::new(state);
    Router::new()
        .route("/health", get(health))
        .route("/capabilities", get(capabilities_route))
        .route("/openapi.json", get(openapi_route))
        .route("/rules", get(rules_route))
        .route("/inspect", post(inspect_route))
        .route("/clean", post(clean_route))
        .fallback(not_found)
        .layer(DefaultBodyLimit::max(max_body_bytes()))
        .layer(middleware::from_fn_with_state(state.clone(), host_guard))
        .with_state(state)
}

#[cfg(test)]
mod tests;
