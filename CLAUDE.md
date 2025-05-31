# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Commands

```bash
# Build the project
cargo build

# Run the application
cargo run

# Build for release
cargo build --release

# Check code without building
cargo check

# Run with optimizations (faster execution)
cargo run --release

# Start database (PostgreSQL)
docker-compose up -d

# Stop database
docker-compose down

# Setup environment files (first time only)
cp api/.env.example api/.env
cp app/.env.example app/.env

# Run API server (requires database to be running)
cargo run -p api

# Run the app
cargo run -p app
```

## Architecture Overview

fast-tag is a Bevy-based image annotation application with a state-driven architecture:

### Core Architecture
- **State Management**: Uses Bevy's state system with `AppState` enum (List/Detail) for page navigation
- **ECS Pattern**: Follows Entity-Component-System architecture via Bevy
- **Async Integration**: Tokio runtime for image downloading, integrated with Bevy's sync systems

### Key Components
- **Main App** (`src/main.rs`): Sets up Bevy app with plugins, states, and system scheduling
- **State Definition** (`src/state.rs`): Defines application states (List, Detail)
- **Page System**: Each page has setup/update/cleanup/ui_system functions following Bevy conventions

### Page Structure
- **List Page** (`src/pages/list.rs`): Main menu with navigation button to Detail page
- **Detail Page** (`src/pages/detail.rs`): Core annotation workspace with image loading and rectangle drawing
- **Common Components** (`src/pages/components/egui_common.rs`): Shared UI components like top navigation panel

### Detail Page Systems
The Detail page implements multiple interaction modes:
- **Drawing Mode**: Click and drag to create new rectangles
- **Grabbing Mode**: Move existing rectangles
- **Resizing Mode**: Resize rectangles by dragging corners
- **Camera Controls**: Zoom (mouse wheel) and pan (right-click drag)

### Resource Management
- **DetailData**: Main resource containing rectangles, interaction state, camera controls
- **Parameters**: Resource for passing data between pages (e.g., image URL)
- **Gizmos**: Used for rendering rectangle overlays with different styles for selected/unselected

### UI Integration
- Uses bevy_egui for immediate mode GUI
- UI systems run on `EguiContextPass` for proper integration
- Cursor icon changes handled through egui context for better UX

### Image Handling
Images are downloaded asynchronously via reqwest and converted to Bevy's Image format for rendering as sprites.