# cargo-skyline

A cargo subcommand for making it easier to work with (and make) [Skyline](https://github.com/ultimate-research/skyline-rs) plugins.

```
cargo-skyline 1.0.0

USAGE:
    cargo skyline <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    build      Build the current plugin as an NRO
    help       Prints this message or the help of the given subcommand(s)
    install    Build the current plugin and install to a switch over FTP
    new        Create a new plugin from a template
    run        Install the current plugin and listen for skyline logging
    set-ip     Set the IP address of the switch to install to
    show-ip    Show the currently configured IP address
```

## Prerequisites

* [Rust](https://www.rust-lang.org/tools/install)
* [git](https://git-scm.com/downloads)

## Installation

```sh
cargo install --git https://github.com/jam1garner/cargo-skyline
```
