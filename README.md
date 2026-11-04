# viel

A terminal client for viel. One screen: type to search movies, TV and anime,
pick something, it plays in mpv. Download anything to your own machine.

viel asks the server which sources carry a title, then streams and downloads
straight from each source's CDN on your PC. A small relay inside the binary
runs on `127.0.0.1`, sends every CDN the headers it expects, unmasks segments
disguised as images, and hands mpv a clean stream. Sources a browser can't
play because of CORS or header checks work here.

## Install

Grab a build from `dist/` or:

```
cargo install --path .
```

Needs [mpv](https://mpv.io) for playback and [ffmpeg](https://ffmpeg.org) for
downloads. viel offers to install whichever is missing when it first needs it.

## Usage

```
viel
```

Start typing. Results update as you type, anime first, then movies and TV.
Before you type anything the list shows what you were watching and what's
trending. An anime with several seasons shows up once, as
`Attack on Titan (2013) · 7 seasons · anime`, and opens a season picker.

Posters render as real images in terminals that support it (Windows Terminal
1.22+, kitty, WezTerm, iTerm2, Ghostty, foot, Konsole) and as colour blocks
everywhere else. `VIEL_IMAGES=off` forces the blocks, `VIEL_IMAGES=sixel`
(or `kitty`, `iterm2`) forces a protocol if detection gets it wrong.

Windows Terminal never reports its cell size, so posters there are sized for
the default 12pt Cascadia Mono at your display scale. With a different font
size, set `image cell size` in settings (or `VIEL_CELL=9x20`) to your real
cell size. `viel --probe` prints what was detected.

| key | home | episodes |
|---|---|---|
| `⏎` | open | play |
| `d` | | download this episode |
| `D` | | download the season, or everything |
| `a` | | switch sub / dub (anime) |
| `^d` | download a movie | |
| `^a` | account | |
| `^s` | settings | |
| `esc` | clear search, then quit | back |

When an episode ends the next one starts after a short countdown; `esc` stops
it and leaves the cursor on the next episode. Coming back from an episode
keeps your search. Playback position syncs with your viel account, so the
website and the CLI both continue where the other left off.

Each title is checked against every source at once, and playback starts on
the best one that answers, in this order:

| movies and TV | anime (sub) | anime (dub) |
|---|---|---|
| cno (up to 4K) | AnimeParadise | AnimeKai |
| mapple | AnimeKai | KotoTV |
| vixsrc | KotoTV | LuffyTV |
| vidbolt (TV only) | LuffyTV | |
| vidrock | | |

If a source fails, the next one takes over. Playback is capped at 1080p by
default (`max quality` in settings goes up to 4K). While a stream loads, the
terminal shows what the relay is fetching and how long it has been waiting.
Anime intros are skipped automatically.

Downloads run three episodes at a time (`parallel downloads` in settings),
eight segments each, under a shared cap of twelve connections so CDNs don't
start rate-limiting. Each finished file is remuxed locally into `.mkv` with the
subtitle in your language embedded. A full episode usually takes one to two
minutes; a ten-episode season a little under five.

## iOS app

`ios/` holds a SwiftUI app (iOS 26, Liquid Glass) built on the same core. The Rust crate in
`mobile/` compiles to a static library that runs the relay and a small JSON
API on `127.0.0.1`; the app plays through AVPlayer with Picture in Picture,
background audio, styled subtitles, intro and outro skip, scrub previews,
auto-advance, offline downloads (kept under Files → viel → Downloads) and
progress sync.

It is sideload only. Every push builds an unsigned `viel.ipa` on GitHub
Actions (`.github/workflows/ios.yml`); install it with Sideloadly or AltStore
using your Apple ID. To build locally on a Mac:

```
rustup target add aarch64-apple-ios
cargo build -p viel-mobile --release --target aarch64-apple-ios
brew install xcodegen
xcodegen generate --spec ios/project.yml
open ios/viel.xcodeproj
```

## Configuration

`~/.config/viel/config.toml`:

```toml
host = "https://viel.tv"
player = "mpv"
download_dir = "/home/you/Videos/viel"
subtitle_lang = "en"
audio = "sub"
parallel_downloads = 3
max_height = 1080
```

`VIEL_HOST` and `VIEL_PLAYER` override the file. Everything is also editable
from settings (`^s`).

## License

MIT
