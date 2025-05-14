# Ion Game Engine

The ultimate, best-in-class, 2.5D game engine written in Rust with cross-platform support.

## Key Features

- **2.5D Graphics**: Modern rendering pipeline with HDR, dynamic lighting, shadows, SSAO etc.
- **Cross-Platform**: Native desktop and web (WASM) support
- **Multi-threaded Architecture**: Separate universe (simulation) and render threads.

## Architecture

This is a ~~hobby project~~ serious upcoming game engine with minimal amount of dependencies. Existing dependencies are mostly just rust abstractions to different OS-specific APIs:

- winit: Unifying api layer for window management across different platforms (windows, mac, x11, wayland, web)
- wgpu: Unifying api layer for rendering between different GPU architectures (metals, directx, vulkan, webgpu)
- Several libraries providing WASM and JS bindings for browser support.
- egui: UI library, because making one yourself is bigger effort than making a game engine.
- bincode: A binary encode/decode library, haven't rolled my own yet.

Everything else made from scratch including math libraries, networking stack, **the whole rendering system**, logging, time/date management, input handling, core architecture etc.

The engine follows a dual-threaded design:

- **Universe Thread**: Handles game simulation, physics, AI, and networking at fixed timestep
- **Render Thread**: Manages graphics rendering, UI, and input at variable refresh rate

## Project Structure

- **`ion_common`**: Shared utilities (math, networking, time management)
- **`ion_engine`**: Core engine (graphics, input, networking, file system)
- **`ion_game`**: Example game implementation using the engine
- **`ion_host`**: Server hosting services (MP support WIP)

## Documentation

See ion_engine project and the lib.rs for entry point into the game engine.
The ion_game crate contains an example implementation utilizing the game engine.

## Quick Start

Before running, add required assets to `ion_game/assets/textures`

### Native Build

```bash
cargo run --bin ion_game
```

### WASM Build, NOTE: Requires nightly rust toolchain

```bash
./scripts/web-run-dev.sh
```
