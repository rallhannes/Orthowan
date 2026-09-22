# Orthowan

Orthowan is a desktop application designed to automatically repair and convert window and door openings in IFC files exported from OrthoGraph. It ensures compatibility with Allplan by converting complex IFCOPENINGELEMENT entities into IFCOPENINGSTANDARDCASE with clean IFCEXTRUDEDAREASOLID geometry.

## Features

- Drag and Drop Interface: Easily load one or multiple IFC files.
- Batch Processing: Processes multiple files sequentially.
- Automated IFC Correction: Projects opening geometries onto the wall axis to create clean, Allplan-compatible extrusions.
- Rename Before Save: Edit the output file names before finalizing the conversion.
- Cross-Platform: Available for Windows, macOS, and Linux.

## Technical Stack

- Backend: Rust (for high performance parsing and file system operations)
- Frontend: HTML, CSS, JavaScript (Vanilla)
- Framework: Tauri

## Development and Building

### Prerequisites

- Rust (via rustup)
- Node.js (optional, if using npm/yarn)
- Tauri CLI (`cargo install tauri-cli`)
- System dependencies for Tauri (e.g., webkit2gtk on Linux, MSVC on Windows)

### Running locally

To run the application in development mode:

```bash
cargo tauri dev
```

### Building Installers

To build the release application and generate installers (.exe for Windows, .deb/.AppImage for Linux, .app/.dmg for macOS):

```bash
cargo tauri build
```

The output bundles will be located in `target/release/bundle/`.

## Architecture

1. The frontend (UI) sends the file paths to the Rust backend.
2. The Rust backend reads the IFC file, identifies `IFCRELVOIDSELEMENT` relations, and calculates the appropriate axis and extrusion depth for the wall.
3. The modified IFC data is written to the system's temporary directory.
4. Once the user clicks "Speichern" and selects a destination folder, the backend moves the temporary files to the final location and renames them accordingly.
