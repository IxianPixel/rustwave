# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Rustwave is a SoundCloud music player built with Rust and the Iced GUI framework. It provides a desktop application for streaming music from SoundCloud with features like playback controls, queue management, and OAuth authentication.

## Development Commands

### Building and Running
- `cargo build --release` - Build optimized release binary
- `cargo run` - Run in development mode
- `./build_app.sh` - Create macOS app bundle (requires .env file and assets)
- `./run_app.sh` - Quick run script
- `open Rustwave.app` - Launch the built macOS app

### Testing and Development
- `cargo check` - Fast syntax and type checking
- `cargo clippy` - Linting and suggestions
- `cargo fmt` - Code formatting

## Architecture

### Core Application Structure
The app uses Iced's MVU (Model-View-Update) pattern with a page-based navigation system:

- **MyApp** (main.rs): Root application managing global playback state, media controls, and page transitions
- **Page trait**: Common interface for different application screens
- **AuthPage**: OAuth login flow for SoundCloud
- **PageB**: Main interface showing tracks, search, and playlist management

### Audio System
- Uses `rodio` for audio playback with `Sink` for stream control
- `souvlaki` for OS media controls integration (play/pause/skip via system controls)
- Custom backward seeking workaround that recreates the audio source when needed

### Queue Management
- **QueueManager** (queue_manager.rs): Handles track queues with next/previous navigation
- **StreamManager** (stream_manager.rs): Downloads and caches audio streams
- Queue starts from selected track and continues through the track list

### API Integration
- **TokenManager** (auth.rs): OAuth2 token management with automatic refresh
- **api_helpers.rs**: SoundCloud API wrapper functions
- **soundcloud.rs**: SoundCloud-specific API endpoints and data handling

### Key Components
- **models.rs**: Data structures for SoundCloud tracks, users, and API responses
- **utilities.rs**: Helper functions for UI widgets, duration formatting, and image downloading
- **config.rs**: Configuration management and environment variable handling
- **constants.rs**: Application constants and default values

### Message Flow
Messages follow a hierarchical pattern:
1. Page-specific messages (PageBMessage, AuthPageMessage)
2. Global app messages (PlayPausePlayback, SeekForwards, etc.)
3. Queue and stream management messages (StartQueue, QueueStreamDownloaded, etc.)

## Configuration

### Environment Variables
Required in `.env` file (copy from `.env.example`):
- `CLIENT_ID`: SoundCloud API client ID
- `CLIENT_SECRET`: SoundCloud API client secret
- `REDIRECT_URL`: OAuth redirect URL (typically http://localhost:5000/)

### Assets
- `assets/icon.png`: Application icon used for app bundle generation
- Icons are automatically resized to various sizes during macOS app bundle creation

## Important Implementation Details

### Audio Playback Quirks
- Backward seeking requires a workaround that recreates the audio source due to rodio limitations
- Track data is cached in memory to enable this backward seeking functionality
- Progress tracking uses a 100ms timer for responsive UI updates

### Token Management
- OAuth tokens are automatically refreshed when expired
- Token manager is passed through async operations to maintain authentication state
- Reauthentication flow redirects back to login page when tokens become invalid

### Platform-Specific Features
- macOS app bundle creation with proper Info.plist and icon generation
- Media controls integration works across different desktop environments
- Window icon loading with fallback handling