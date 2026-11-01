# Selenite Lamp Updater

Schedule and count driven updater for the
[selenite-lamp](https://github.com/julianandrews/selenite-lamp) `watch-file`
mode. It writes the command that `selenite-lamp watch-file` should send as
JSON, based on cron-style timers and watched count files.

## Build and run

```
nix build .#selenite-lamp-updater
nix run .# -- -c config.toml "${XDG_RUNTIME_DIR}/selenite-lamp.json"
```

The output file is the optional positional argument; it can also be set with
`output-file` in the config.

## Config

The config is TOML. Mode order is priority order: the last active mode wins.

```toml
default-mode = "Resting"

[[modes]]
name = "Resting"
command = { gleam = { frequency = 0.006, num_groups = 4, period = 10000 } }

[[modes]]
name = "Email"
command = { pulse-hue = { hue = 40000, period = 2000, wait = 0 } }

[[modes]]
name = "Night"
command = { pulse-hue = { hue = 52000, period = 10000, wait = 20000 } }

[[timers]]
schedule = "0 0 0 * * * *" # sec min hour day month weekday year
duration = 28800
mode = "Night"

[[count-files]]
mode = "Email"
file = "/home/julian/.cache/selenite-lamp/email-count"
```

Count files contain a single number; any positive value enables the mode.
