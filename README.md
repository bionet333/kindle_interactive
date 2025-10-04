# E-Ink Sync: Desktop Editor & Web Reader

This application provides a desktop interface built with Tauri and Yew to edit Markdown text. The text is served over the local network via a lightweight, embedded Rust web server, optimized for e-ink reading devices.

The core functionality has been implemented in a high-performance, concurrent Rust backend, providing a seamless, native experience.

## Core Features

- **Native Desktop Editor**: A clean, simple Markdown editor built with Yew (WASM) and Rust.
- **Embedded Web Server**: A high-performance Axum web server serves the content to devices on the same network.
- **Optimized for E-Ink**: The reader webpage (`/get`) has zero animations and client-side pagination for instant page turns.
- **Live Updates**: The e-ink reader automatically polls for content changes, ensuring it's always in sync with the editor.

## Architecture Overview

The application follows a clean, decoupled architecture:
1.  **Tauri Shell**: The native desktop window and process manager.
2.  **Yew Frontend (WASM)**: The user interface running inside the Tauri WebView. It communicates with the backend via Tauri's `invoke` API.
3.  **Rust Backend**:
    - **Tauri Commands**: A set of functions (`get_text`, `set_text`) that act as the API layer for the frontend.
    - **Shared State**: A thread-safe, in-memory store (`AppState`) for the Markdown content.
    - **Axum Web Server**: An embedded web server that runs in a background thread, reading from the shared state to serve content to e-ink devices on the local network.

## Development Setup

### Prerequisites

- [Rust and Cargo](https://www.rust-lang.org/tools/install)
- [Node.js](https://nodejs.org/en/) (for Tauri's internal web dependencies)
- [Trunk](https://trunkrs.dev/#install)
- Tauri CLI: `cargo install tauri-cli`

### Running the Application

1.  **Run in development mode**:
    ```sh
    cargo tauri dev
    ```

This command will start the Tauri application, including the Yew frontend and the Rust backend server.

### Building for Production

To build a distributable, native application, run:

```sh
 cargo tauri build
```

## Logging and Diagnostics

The application includes a comprehensive logging setup (`tauri-plugin-log`) to aid in debugging and monitoring. Logs are crucial for diagnosing issues like content not updating on the reader device.

- **Backend Logs**: Viewable in the terminal where you run `cargo tauri dev`. They show server status, requests from the reader, and state updates from the editor.
- **Frontend Logs**: Viewable in the WebView's developer console (press F12 or right-click -> Inspect).
- **File Logs**: Persisted in the application's log directory for post-mortem analysis.

This setup allows developers to trace the entire data flow from a button click in the UI to a content update request from the e-ink device.
