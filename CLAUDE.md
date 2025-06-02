# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

fast-tag is a Bevy-based image annotation application designed for creating annotations for machine learning purposes. The application features a state-driven architecture and introduces a "Project" concept that serves as a container to organize and manage multiple annotation groups, providing structure for large-scale annotation workflows.

## Code Guidelines

- Write source code and comments primarily in English
- Follow Rust best practices and conventions
- Use Bevy's ECS patterns consistently

## Development Setup

### Environment Setup (First Time Only)
```bash
# Copy environment files
cp api/.env.example api/.env
cp app/.env.example app/.env

# Start database (PostgreSQL)
docker compose up
```

### Development Commands
```bash
# Build the project
cargo build

# Run the application
cargo run -p app

# Run API server (requires database)
cargo run -p api

# Check code without building
cargo check

# Build for release
cargo build --release

# Run with optimizations
cargo run --release
```

### Database Management
```bash
# Start database
docker compose up

# Stop database
docker compose down
```

## Code Quality

Always run these commands and fix error and warnings after making source code changes:

```bash
cargo check -p api
cargo check -p app
```

## Architecture

### Core Architecture
- **State Management**: Uses Bevy's state system with `AppState` enum for page navigation (List/Detail)
- **ECS Pattern**: Follows Entity-Component-System architecture via Bevy framework
- **Async Integration**: Tokio runtime for image downloading, seamlessly integrated with Bevy's synchronous systems

### Application Structure
- **Main App** (`src/main.rs`): Configures Bevy app with plugins, states, and system scheduling
- **State Definition** (`src/app/state.rs`): Defines application states and transitions
- **Page System**: Each page implements setup/update/cleanup/ui_system functions following Bevy conventions

### Pages
- **List Page** (`src/pages/list.rs`): Main menu interface with navigation to Detail page
- **Detail Page** (`src/pages/detail.rs`): Core annotation workspace featuring image loading and rectangle drawing capabilities
- **Login Page** (`src/pages/login.rs`): Authentication interface
- **Common Components** (`src/ui/components/`): Shared UI components including navigation panels

### Interaction Systems

The Detail page supports multiple interaction modes:
- **Drawing Mode**: Click and drag to create new annotation rectangles
- **Grabbing Mode**: Move existing rectangles by dragging
- **Resizing Mode**: Resize rectangles by dragging corner handles
- **Camera Controls**: Zoom with mouse wheel, pan with right-click drag

### Resource Management
- **DetailData**: Primary resource containing rectangles, interaction state, and camera controls
- **Parameters**: Resource for data transfer between pages (e.g., image URLs)
- **Gizmos**: Rendering system for rectangle overlays with visual distinction for selected/unselected states

### UI Integration
- **bevy_egui**: Immediate mode GUI integration for user interface elements
- **EguiContextPass**: Proper system scheduling for UI rendering
- **Cursor Management**: Dynamic cursor icon changes through egui context for enhanced user experience

### Image Processing
Images are downloaded asynchronously using reqwest and converted to Bevy's Image format for sprite rendering, enabling efficient display and manipulation within the annotation workspace.