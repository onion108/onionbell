# Onion Bell 🔔

A utility to handle `xdg-system-bell-v1` in Hyprland.

## Usage

Create a config file in `$XDG_CONFIG_HOME/onionbell/config.toml` with content:
```toml
sound = "/path/to/your/sound/file"
```

And try, for example, `printf "\a"` in kitty. You should hear the sound play. If it doesn't, check the logs.

## Rules
You can write several rules to use different sound for different windows. For example, a config file like this
```toml
sound = "/path/to/sound_file1.wav"

[[rule]]
floating = true
sound = "/path/to/sound_file2.wav"

[[rule]]
class_regex = "^kitty$"
sound = "/path/to/sound_file3.wav"
```

Will make `/path/to/sound_file2.wav` to be played on all floating windows that sends a bell event, and `/path/to/sound_file3.wav` to be played on all non-floating `kitty` windows that sends a bell event, and `/path/to/sound_file1.wav` on all other windows that sends a bell event. Notice that rules are executed in order and the first match will be used.

`sound` keys can be absent. In that case, no sound will be played.

## Build

```bash
cargo build --release
```

