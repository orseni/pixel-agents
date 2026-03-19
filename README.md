<h1 align="center">
    <img src="webview-ui/public/banner.png" alt="Pixel Agents">
</h1>

<h2 align="center" style="padding-bottom: 20px;">
  The game interface where AI agents build real things
</h2>

<br/>

> **This is a fork.** The [original Pixel Agents](https://github.com/pablodelucca/pixel-agents) is a VS Code extension — it only works inside VS Code and requires you to launch Claude Code terminals from within the editor.
>
> **This fork is a standalone desktop app** built with [Tauri](https://tauri.app/). It runs independently of any editor, passively monitors **all** your Claude Code sessions across your entire machine, and shows each one as an animated character in real time. You don't need VS Code open. You don't need to click "+ Agent". Just run Claude Code anywhere — in any terminal, in any project — and the characters appear automatically.

<br/>

## Why this fork exists

The original extension is great, but it's tied to VS Code. If you use Claude Code from a standalone terminal, from Cursor, from Warp, or from multiple VS Code windows at once, the extension can't see all of them.

This fork solves that by watching `~/.claude/projects/` directly. Every active Claude Code session on your machine shows up as a character, regardless of where it's running. The app sits in its own window and gives you a birds-eye view of everything your agents are doing.

**Key differences from the original:**

| | Original (VS Code extension) | This fork (Tauri standalone) |
|---|---|---|
| **Runtime** | VS Code webview panel | Native desktop app (Tauri + Rust) |
| **Detection** | Only terminals opened inside VS Code | All Claude Code sessions on the machine |
| **Agent creation** | Manual (click "+ Agent") | Automatic (monitors `~/.claude/projects/`) |
| **Editor dependency** | Requires VS Code | None — works with any terminal |
| **Backend** | Node.js + VS Code API | Rust (async, low resource usage) |

![Pixel Agents screenshot](webview-ui/public/Screenshot.jpg)

## Features

Everything from the original, plus standalone operation:

- **Automatic agent discovery** — every active Claude Code session becomes a character, no manual setup
- **Editor-independent** — works with any terminal: iTerm, Warp, Alacritty, VS Code, Cursor, etc.
- **Project name detection** — each character is labeled with its project name, extracted from the session path
- **Live activity tracking** — characters animate based on what the agent is actually doing (writing, reading, running commands)
- **Office layout editor** — design your office with floors, walls, and furniture using a built-in editor
- **Speech bubbles** — visual indicators when an agent is waiting for input or needs permission
- **Sound notifications** — optional chime when an agent finishes its turn
- **Sub-agent visualization** — Task tool sub-agents spawn as separate characters linked to their parent
- **Persistent layouts** — your office design is saved across sessions
- **Diverse characters** — 6 diverse characters based on the work of [JIK-A-4, Metro City](https://jik-a-4.itch.io/metrocity-free-topdown-character-pack)

<p align="center">
  <img src="webview-ui/public/characters.png" alt="Pixel Agents characters" width="320" height="72" style="image-rendering: pixelated;">
</p>

## Requirements

- macOS, Linux, or Windows
- [Claude Code CLI](https://docs.anthropic.com/en/docs/claude-code) installed and configured
- [Rust toolchain](https://rustup.rs/) (for building from source)

## Getting Started

### Install from source

```bash
git clone https://github.com/YOUR_USERNAME/pixel-agents.git
cd pixel-agents
npm install
cd webview-ui && npm install && cd ..
npm run build
```

The compiled binary will be at `src-tauri/target/release/pixel-agents`.

### Development

```bash
npm run dev
```

This starts the Vite dev server and the Tauri app with hot-reload.

### Usage

1. Launch the app — it opens a standalone window with the pixel art office
2. Open Claude Code in **any terminal** on your machine
3. Watch the character appear automatically and animate in real time
4. Click a character to select it, then click a seat to reassign it
5. Click **Layout** to open the office editor and customize your space

## Layout Editor

The built-in editor lets you design your office:

- **Floor** — Full HSB color control
- **Walls** — Auto-tiling walls with color customization
- **Tools** — Select, paint, erase, place, eyedropper, pick
- **Undo/Redo** — 50 levels with Ctrl+Z / Ctrl+Y
- **Export/Import** — Share layouts as JSON files via the Settings modal

The grid is expandable up to 64×64 tiles. Click the ghost border outside the current grid to grow it.

### Office Assets

All office assets (furniture, floors, walls) are now **fully open-source** and included in this repository under `webview-ui/public/assets/`. No external purchases or imports are needed — everything works out of the box.

Each furniture item lives in its own folder under `assets/furniture/` with a `manifest.json` that declares its sprites, rotation groups, state groups (on/off), and animation frames. Floor tiles are individual PNGs in `assets/floors/`, and wall tile sets are in `assets/walls/`. This modular structure makes it easy to add, remove, or modify assets without touching any code.

To add a new furniture item, create a folder in `webview-ui/public/assets/furniture/` with your PNG sprite(s) and a `manifest.json`, then rebuild. The asset manager (`scripts/asset-manager.html`) provides a visual editor for creating and editing manifests.

Detailed documentation on the manifest format and asset pipeline is coming soon.

Characters are based on the amazing work of [JIK-A-4, Metro City](https://jik-a-4.itch.io/metrocity-free-topdown-character-pack).

## How It Works

Every 5 seconds, the Rust backend scans `~/.claude/projects/` for JSONL transcript files that were recently modified. Each active file becomes an agent. The backend reads new lines incrementally, parses tool_use/tool_result/turn_duration records, and emits events to the frontend via Tauri IPC.

The frontend is a React app running inside a native Tauri window. It renders a pixel art office with a canvas-based game loop, BFS pathfinding, and a character state machine (idle, walk, type, read). Characters animate based on what the agent is actually doing. No modifications to Claude Code are needed — it's purely observational.

## Tech Stack

- **Backend**: Rust, Tauri v2, Tokio (async runtime), notify (file watching)
- **Frontend**: React 19, TypeScript, Vite, Canvas 2D
- **IPC**: Tauri events (backend to frontend) + Tauri commands (frontend to backend)

## Known Limitations

- **Heuristic-based status detection** — Claude Code's JSONL transcript format does not provide clear signals for when an agent is waiting for user input or when it has finished its turn. The current detection is based on heuristics (idle timers, turn-duration events) and may occasionally misfire.
- **Session detection delay** — new sessions are detected within 5 seconds. Sessions are considered inactive after 5 minutes without JSONL writes.

## Where This Is Going

The long-term vision is an interface where managing AI agents feels like playing the Sims, but the results are real things built.

- **Agents as characters** you can see, assign, monitor, and redirect, each with visible roles (designer, coder, writer, reviewer), stats, context usage, and tools.
- **Desks as directories** — drag an agent to a desk to assign it to a project or working directory.
- **An office as a project** — with a Kanban board on the wall where idle agents can pick up tasks autonomously.
- **Deep inspection** — click any agent to see its model, branch, system prompt, and full work history. Interrupt it, chat with it, or redirect it.
- **Token health bars** — rate limits and context windows visualized as in-game stats.
- **Fully customizable** — upload your own character sprites, themes, and office assets. Eventually maybe even move beyond pixel art into 3D or VR.

For this to work, the architecture needs to be modular at every level:

- **Platform-agnostic**: VS Code extension today, Electron app, web app, or any other host environment tomorrow.
- **Agent-agnostic**: Claude Code today, but built to support Codex, OpenCode, Gemini, Cursor, Copilot, and others through composable adapters.
- **Theme-agnostic**: community-created assets, skins, and themes from any contributor.

We're actively working on the core module and adapter architecture that makes this possible. If you're interested to talk about this further, please visit our [Discussions Section](https://github.com/pablodelucca/pixel-agents/discussions).


## Community & Contributing

We use **[GitHub Discussions](https://github.com/pablodelucca/pixel-agents/discussions)** for questions, feature ideas, and conversations. **[Issues](https://github.com/pablodelucca/pixel-agents/issues)** are for bug reports only.

If something is broken, open an issue. For everything else, start a discussion.

See [CONTRIBUTING.md](CONTRIBUTING.md) for instructions on how to contribute.

Please read our [Code of Conduct](CODE_OF_CONDUCT.md) before participating.

## Supporting the Project

If you find Pixel Agents useful, consider supporting its development:

<a href="https://github.com/sponsors/pablodelucca">
  <img src="https://img.shields.io/badge/Sponsor-GitHub-ea4aaa?logo=github" alt="GitHub Sponsors">
</a>
<a href="https://ko-fi.com/pablodelucca">
  <img src="https://img.shields.io/badge/Support-Ko--fi-ff5e5b?logo=ko-fi" alt="Ko-fi">
</a>

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=pablodelucca/pixel-agents&type=Date)](https://www.star-history.com/?repos=pablodelucca%2Fpixel-agents&type=date&legend=bottom-right)

## License

This project is licensed under the [MIT License](LICENSE).
