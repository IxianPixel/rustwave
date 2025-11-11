<p align="center">
    <img align="center" style="margin: 10px;" width="250px" src="./assets/icon.png"/>
<br>
<a href="https://github.com/IxianPixel/rustwave/releases/latest">
    <img src="https://img.shields.io/github/v/release/IxianPixel/rustwave" alt="Version">
</a>
<a href="https://github.com/IxianPixel/rustwave/actions/workflows/ci.yml">
    <img src="https://github.com/IxianPixel/rustwave/actions/workflows/ci.yml/badge.svg" alt="CI">
</a>
<a href="https://github.com/IxianPixel/rustwave/actions/workflows/release.yml">
    <img src="https://github.com/IxianPixel/rustwave/actions/workflows/release.yml/badge.svg" alt="Build Status">
</a>
</p>

## Table of Contents

- [About](#about)
- [Installation](#installation)
- [Screenshots](#screenshots)
- [Acknowledgement](#acknowledgement)

## About

Rustwave is a SoundCloud client written entirely in Rust. It is still under heavy development with features being added
regularly. The goal is to reach feature parity with the official SoundCloud. The current feature set includes:

- Loading Feed
- Loading Likes
- Searching
    - Tracks
    - Users
    - Playlists
- Liking Tracks
- Playing Tracks
- Full Integration with OS Media Controls

## Installation

### Requirements

A SoundCloud account is **required**

#### Dependencies

##### Windows and MacOS

- [Rust and cargo](https://www.rust-lang.org/tools/install) as the build dependencies

##### Linux

- [Rust and cargo](https://www.rust-lang.org/tools/install) as the build dependencies
- Install `openssl`, `alsa-lib` (`streaming` feature), `libdbus` (`media-control` feature).

    - For example, on Debian based systems, run the below command to install application's dependencies:

      ```shell
      sudo apt install libssl-dev libasound2-dev libdbus-1-dev
      ```

    - On RHEL/Fedora based systems, run the below command to install application's dependencies :

      ```shell
      sudo dnf install openssl-devel alsa-lib-devel dbus-devel
      ```

      or if you're using `yum`:

      ```shell
      sudo yum install openssl-devel alsa-lib-devel dbus-devel
      ```

### Building

At the moment you need to clone to repo and build it yourself.

#### MacOS

- A build script is provided to build the application on macOS. This will generate an application bundle in the root
  directory.

    ```shell
    git clone https://github.com/IxianPixel/rustwave.git
    cd rustwave
    ./build_app.sh
    ```

#### Windows and Linux

- Build the application using cargo. This will generate an executable in the `target/release` directory.

    ```shell
    git clone https://github.com/IxianPixel/rustwave.git
    cd rustwave
    cargo build --release
    ```

## Screenshots

### Search

<p align="center">
    <img align="center" width="600px" src="./assets/screenshots/search.png"/>
</p>

### Feed

<p align="center">
    <img align="center" width="600px" src="./assets/screenshots/feed.png"/>
</p>

## Acknowledgement

Rustwave is written in [Rust](https://www.rust-lang.org) and is built on top of awesome libraries such
as [Iced](https://github.com/iced-rs/iced), [rodio](https://github.com/RustAudio/rodio), [souvlaki](https://github.com/Sinono3/souvlaki),
and [many more](Cargo.toml).
