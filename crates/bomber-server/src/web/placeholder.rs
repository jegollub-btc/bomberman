//! What you get at `/` before anyone has built a visualizer.
//!
//! A bare 404 here reads as a broken server. This says what is running, what is
//! missing, and where the contract for building it lives.

use axum::response::Html;

pub async fn page() -> Html<&'static str> {
    Html(PAGE)
}

const PAGE: &str = r#"<!doctype html>
<meta charset="utf-8">
<title>Bomberman Arena</title>
<style>
  :root { color-scheme: light dark; }
  body { font: 16px/1.6 system-ui, sans-serif; max-width: 44rem; margin: 4rem auto; padding: 0 1.5rem; }
  h1 { font-size: 1.5rem; margin-bottom: .25rem; }
  p.lede { color: #777; margin-top: 0; }
  code { background: #8882; padding: .1em .35em; border-radius: .25em; }
  table { border-collapse: collapse; margin: 1.5rem 0; width: 100%; }
  td, th { text-align: left; padding: .4rem .75rem .4rem 0; border-bottom: 1px solid #8883; }
  ul { padding-left: 1.2rem; }
</style>
<h1>Bomberman Arena</h1>
<p class="lede">The server is running. No visualizer has been built yet.</p>

<p>The arena is live and accepting bots. What is missing is the front end, which
is a separate deliverable &mdash; drop a build into
<code>visualizer/dist</code> and it will be served here instead of this page.</p>

<table>
  <tr><th>Endpoint</th><th>What it is</th></tr>
  <tr><td><code>udp/47800</code></td><td>Bots. Two bytes up, binary frames down.</td></tr>
  <tr><td><code>/ws/spectate</code></td><td>Read-only match feed, JSON, 60&nbsp;per second.</td></tr>
  <tr><td><code>/ws/admin</code></td><td>The same feed, plus start / pause / end.</td></tr>
</table>

<p>The contracts are written up in the repository:</p>
<ul>
  <li><code>BOT_GUIDE.md</code> &mdash; for anyone writing a bot.</li>
  <li><code>VISUALIZER_GUIDE.md</code> &mdash; for whoever builds this page properly.</li>
  <li><code>MODERATION_API.md</code> &mdash; start, pause and end.</li>
</ul>
"#;
