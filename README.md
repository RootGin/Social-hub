# Social Hub

A desktop application for managing multiple social media accounts in one place. Each account opens in its own isolated webview session with persistent cookies, allowing users to switch between platforms without logging in/out repeatedly.

Built with **Tauri v2** + **Svelte 4** + **WebKitGTK** — cross-platform (Linux, macOS, Windows).

## Use Cases

### Multi-Account Social Media Management

Users who manage multiple social media presences (personal, work, side projects) can keep each account logged in simultaneously in separate isolated sessions. No more browser profile switching or "log out of one to check the other."

### Platform Support

Each platform runs in its own webview with session isolation:

| Platform | Purpose |
|---|---|
| **Zalo** | Primary messaging for Vietnamese users |
| **Twitter / X** | Real-time news, professional networking, content scheduling |
| **Instagram** | Visual content management, brand engagement |
| **Facebook** | Community management, page administration |
| **TikTok** | Short-form video content monitoring |

### Session Isolation

Every account gets its own browser context (separate cookie store, localStorage, cache). This means:
- A user can be logged into 3 Twitter accounts simultaneously
- Logging out of one account doesn't affect others
- Each session persists across app restarts

### Why a Desktop App

- No tab clutter — each account is its own window
- Keyboard-driven navigation between accounts
- Lower memory footprint than running multiple browser profiles
- Native OS integration (taskbar per account window, system notifications)

## Building from Source

```bash
# Prerequisites: Rust, Node.js, WebKitGTK 4.1
git clone https://github.com/RootGin/Social-hub.git
cd Social-hub

# Install JS dependencies
npm install

# Development mode (hot reload)
npm run tauri dev

# Production build
npm run tauri build
```

### NixOS

```bash
nix develop --command npm run tauri dev
```

## Project Structure

```
Social-hub/
├── src/                  # Svelte frontend
│   ├── App.svelte        # Main dashboard UI
│   ├── app.css           # Global reset & base styles
│   └── main.js           # Svelte entry point
├── src-tauri/            # Rust backend
│   ├── src/
│   │   ├── main.rs       # App entry, window creation
│   │   ├── commands.rs   # Tauri IPC commands
│   │   └── state.rs      # Config, session state
│   ├── capabilities/     # Tauri v2 permission manifests
│   └── tauri.conf.json   # Tauri configuration
├── index.html            # Svelte mount point
├── vite.config.js        # Vite + Svelte config
└── flake.nix             # Nix development shell
```

## License

This is free and unencumbered software released into the public domain under [The Unlicense](https://unlicense.org/).
