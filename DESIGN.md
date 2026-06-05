# viel-cli

A terminal client for viel. Search and watch movies, TV, and anime from the
shell; download streams; keep watch history in sync with the web app. Runs on
Linux, macOS, and Windows as a single binary.

## Why it exists

The viel server already does the hard part: provider extraction, the
Referer-forging proxy, CDN bypass, the videasy WASM patch. All of it sits behind
plain HTTP endpoints. The CLI reimplements none of that — it is a thin client
that calls the same JSON API the web player uses and hands the resolved stream to
mpv. When a provider rotates, the server is fixed once and both clients keep
working.

## Non-goals

- No extraction logic in the client. If the server can't resolve it, the CLI
  can't either.
- No bundled player. mpv (or VLC) must be installed; the CLI shells out to it.
- Not tied to one host. The base URL is configurable so it can point at any
  mirror.

## Architecture

```
                 viel-cli (one binary)
   ┌───────────────┬───────────────┬──────────────┐
   │  interactive  │   commands    │  tray daemon  │
   │   home menu   │ search/play/  │ (--tray)      │
   │               │ dl/continue   │               │
   └───────┬───────┴───────┬───────┴──────┬────────┘
           │ HTTPS          │              │ local IPC
           ▼                ▼              ▼
     stream-web API     mpv process    Discord IPC pipe
   /proxy/* /miruro/*  (--input-ipc)
   /api/me/*
```

The binary has three faces sharing one codebase:

- **interactive** — run `viel` with no args: the home menu.
- **commands** — `viel search`, `viel play`, `viel dl`, `viel continue` for
  scripting and quick access.
- **tray daemon** — `viel --tray`: a resident helper that provides Discord Rich
  Presence for people watching in the browser (see Discord RPC).

## Endpoints consumed

| Purpose | Endpoint |
|---|---|
| Search | `GET /proxy/tmdb/search/multi?query=` |
| Trending (random) | `GET /proxy/tmdb/trending/all/week` |
| Seasons | `GET /proxy/tmdb/tv/{id}` |
| Episodes | `GET /proxy/tmdb/tv/{id}/season/{n}` |
| Resolve stream | `GET /proxy/extractor/{src}/{id}[/{s}/{e}]` |
| Subtitles | `GET /proxy/videasy-subs/{id}[/{s}/{e}]` |
| Anime search | `GET /miruro/search?query=` |
| Anime episodes | `GET /miruro/episodes/{anilistId}` |
| Anime sources | `GET /miruro/sources?episodeId=&provider=&anilistId=` |
| History read | `GET /api/me/history` |
| History write | `POST /api/me/history` |
| Skip timestamps | `GET /api/skip/{id}/{s}/{e}` |

## Stream resolution

For movies and TV the client walks the same cascade as the web player —
`vidrock → vidzee → videasy → anyembed` — taking the first response with a
playable URL. The extractor response carries the `Referer` the CDN demands; the
client passes it straight to mpv. No `/proxy/manifest` round trip is needed for
playback because mpv speaks HLS and sends the header itself.

```
mpv "<m3u8>" --http-header-fields="Referer: https://vidrock.net/" \
    --force-media-title="Severance · S2E1"
```

## Playback and shortcuts

mpv already provides mute, volume, seek, pause, and fullscreen. The CLI ships an
`input.conf` adding viel-specific binds: next episode, skip intro. On a headless
box wired to a TV, mpv renders to the framebuffer with no X server via
`--vo=gpu --gpu-context=drm`.

Next episode without relaunch: mpv is started with
`--input-ipc-server=<socket>`. A keybind emits `script-message viel-next`; the
CLI, listening on the socket, resolves the next episode and sends `loadfile` back
down it. The stream swaps in place.

## History sync

Bidirectional and free. Both clients authenticate against the same account and
hit the same `/api/me/history`. Watching in the terminal writes progress that
shows up in the web Continue-Watching list and vice versa.

The one moving part: while mpv plays, the CLI polls `time-pos` over the IPC
socket every ten seconds and POSTs `{tmdbId, season, episode, position,
duration}` — the same payload the web player sends. A final POST fires on exit.

## Discord Rich Presence

In the terminal it is native: the CLI talks to the local Discord IPC pipe via the
`discord-rich-presence` crate and sets presence while mpv plays.

For browser viewing, presence needs a local process, because a website cannot
reach the Discord pipe and the server cannot reach a NATed client. The tray
daemon solves this:

- `viel --tray` runs a small HTTP server on `127.0.0.1`.
- The web page POSTs now-playing directly to `http://127.0.0.1:<port>`. Browsers
  permit `https://viel.tv → http://127.0.0.1` under the localhost exception, so
  no backend involvement and no server load.
- The daemon sets Discord presence locally.

The daemon must check the `Origin` header and accept only viel.tv and its
mirrors, otherwise any visited site could spoof presence. A socket.io reverse
channel (daemon connects outbound, joins a room keyed to the account) is an
optional fallback for watching on a different machine than Discord runs on.

The daemon minimizes to the system tray and is quit from the tray menu, the same
way Discord behaves.

## Configuration

`~/.config/viel/config.toml` (platform-appropriate via `directories`):

```toml
host = "https://viel.tv"
player = "mpv"
```

`VIEL_HOST` overrides the host. Session token, once login lands, is stored
alongside the config.

## Crates

| Concern | Crate |
|---|---|
| Prompts | `inquire` |
| Terminal images | `viuer` |
| HTTP | `reqwest` (blocking) |
| JSON | `serde`, `serde_json` |
| Config paths | `directories` |
| Config format | `toml` |
| Discord RPC | `discord-rich-presence` |
| Tray | `tray-icon` |

## Distribution

A single static binary per platform. `cargo install viel`, plus Homebrew and
Scoop manifests so it is `brew install viel` / `scoop install viel`.

## Milestones

1. Home menu, movie and TV search, resolve cascade, play in mpv.
2. Posters via `viuer`; season and episode navigation.
3. Login; history read and write; continue watching.
4. Anime via the miruro endpoints.
5. Download with segment retry and progress.
6. Discord RPC in the terminal.
7. Tray daemon and the browser bridge.
