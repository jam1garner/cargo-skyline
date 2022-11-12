# cargo-skyline

A cargo subcommand for making it easier to work with (and make) [Skyline](https://github.com/ultimate-research/skyline-rs) plugins.

```
cargo-skyline 3.2.0

USAGE:
    cargo skyline <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    build            Build the current plugin as an NRO
    check            Check if the current plugin builds and emit any errors found
    clean-project    Clean a pre-existing project files no longer needed for the latest version
    clippy           Emit beginner-helpful lints and warnings
    cp               Copy a file over FTP
    doc              Document the current plugin and its dependencies
    help             Prints this message or the help of the given subcommand(s)
    install          Build the current plugin and install to a switch over FTP
    list             List the files in the plugin directory for the given game
    listen           Listen for logs being output from a switch running skyline at the given ip
    new              Create a new plugin from a template
    package          Package plugin and latest Skyline into a zip file to prepare it for release
    restart          Install the current plugin and listen for skyline logging
    restart-game     Restart the given game using restart-plugin
    rm               Delete a file in the plugin directory for the given game
    run              Install the current plugin and listen for skyline logging
    self-update      Update cargo-skyline command
    set-ip           Set the IP address of the switch to install to
    show-ip          Show the currently configured IP address
    update           Update libraries for current plugin folder
    update-std       Download the latest stdlib for aarch64-skyline-switch
```

## Prerequisites

* [Rust](https://www.rust-lang.org/tools/install)
* [git](https://git-scm.com/downloads)

## Installation

```sh
cargo install cargo-skyline
```

## Example usage

Create a new plugin called `fps_counter` (in a folder of the same name) in the current directory:
```
cargo skyline new fps_counter
```

Build the current plugin as an nro:
```
cargo skyline build
```

Set the ip of the Switch to install to as `192.168.0.0`:
```
cargo skyline set-ip 192.168.0.0
```

Install the current plugin on a switch at ip `192.168.0.0` for an application with title of `01006A800016E000`:
```
cargo skyline install --ip 192.168.0.0 --title-id 01006A800016E000
```
Note: if the IP has been set, it can be omitted from the arguments (or overriden using the arguments).

To set a default title id for a plugin use the following format in `Cargo.toml`:
```toml
[package.metadata.skyline]
titleid = "01006A800016E000"
```

Install the current plugin to the default IP and title ID, then listen for output from the console:
```
cargo skyline run
```

## Alternative Installation via Docker

```sh
docker build -t cargo-skyline .
docker run it cargo-skyline cargo skyline --help
```
