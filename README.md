# viel

A terminal client for viel. Search and watch movies, TV, and anime from the
shell, with playback through mpv.

## Install

```
cargo install --path .
```

Requires [mpv](https://mpv.io) on your PATH.

## Usage

```
viel
```

Pick movies, TV, or random from the home menu, search, choose a result, and it
plays in mpv. For TV, choose a season and episode.

## Configuration

`~/.config/viel/config.toml`:

```toml
host = "https://viel.tv"
player = "mpv"
```

`VIEL_HOST` and `VIEL_PLAYER` override the file.

## Status

Early. The home menu, movie and TV search, and mpv playback work. Posters,
anime, login with cross-device history, downloads, and Discord presence are on
the roadmap in [DESIGN.md](DESIGN.md).

## License

MIT
