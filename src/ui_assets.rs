//! Serves the built Svelte UI (`web/ui/dist`) in the `full` build. Embedded
//! into the binary at compile time so the router still ships as one file.

use axum::body::Body;
use axum::http::{StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "web/ui/dist/"]
struct Assets;

/// Shaka Player and its controls stylesheet are loaded on demand by the web UI
/// for HLS, DASH and Widevine playback. They stay as a separate embed at
/// `/static/external/...` — the same path the Go version served them at — so
/// updating the player does not require bundling it into the Vite app.
#[derive(RustEmbed)]
#[folder = "web/static/"]
struct StaticAssets;

/// `/` serves `index.html` (its own open route, like `app.Get("/", ...)` in
/// `cmd/ui.go`); `/ui/<path>` serves the built assets it references (its own
/// open route, like `app.Use("/ui", filesystem...)`); `/static/<path>`
/// serves the vendored player libraries above. Everything else falls
/// through to a plain 404, matching the Go build's behaviour of not treating
/// unknown top-level paths as SPA routes.
pub async fn serve_ui(uri: Uri) -> Response {
    let path = uri.path();
    if let Some(rest) = path.strip_prefix("/static/") {
        return match StaticAssets::get(rest) {
            Some(file) => respond(rest, file.data.into_owned()),
            None => not_found(),
        };
    }
    let asset_path = if path == "/" {
        "index.html"
    } else if let Some(rest) = path.strip_prefix("/ui/") {
        rest
    } else {
        return not_found();
    };

    match Assets::get(asset_path) {
        Some(file) => respond(asset_path, file.data.into_owned()),
        None => not_found(),
    }
}

/// `index.html` names the hashed bundle, so it must be revalidated on every
/// load: a cached copy keeps pointing at the previous build after an update.
/// Everything Vite content-hashes under `assets/` can never change under the
/// same name, so it is cached for a year; the rest gets a short cache.
fn cache_control(path: &str) -> &'static str {
    if path == "index.html" {
        "no-cache"
    } else if path.starts_with("assets/") {
        "public, max-age=31536000, immutable"
    } else {
        "public, max-age=3600"
    }
}

fn not_found() -> Response {
    (StatusCode::NOT_FOUND, "not found").into_response()
}

fn respond(path: &str, data: Vec<u8>) -> Response {
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime.as_ref())
        .header(header::CACHE_CONTROL, cache_control(path))
        .body(Body::from(data))
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::cache_control;

    #[test]
    fn index_is_revalidated_but_hashed_assets_are_immutable() {
        assert_eq!(cache_control("index.html"), "no-cache");
        assert_eq!(
            cache_control("assets/index-abc123.js"),
            "public, max-age=31536000, immutable"
        );
        assert_eq!(cache_control("favicon.ico"), "public, max-age=3600");
    }
}
