# Pixel Agents Standalone: Monitoring Every Claude Code Session as Pixel Art Characters

There's something oddly satisfying about watching your AI agents work. Not the terminal output scrolling by, but actually *seeing* them as little characters sitting at desks, typing away, walking around an office.

That's the whole idea behind Pixel Agents. It turns your Claude Code sessions into animated pixel art characters living in a tiny office. It's fun. It looks great. And once you have it running, you'll find yourself glancing at it way more than you expected.

![SCREENSHOT: The standalone Pixel Agents app showing the pixel art office with multiple characters at desks, each labeled with its project name. A few characters are typing, one is idle. The app runs in its own window. Caption: "Every active Claude Code session on the machine, visualized as pixel art characters in a shared office."]

## The Original Project

[Pixel Agents](https://github.com/pablodelucca/pixel-agents) is a VS Code extension created by Pablo de Lucca. The concept is simple and honestly kind of brilliant: every Claude Code terminal you open gets its own animated character in a tiny pixel art office. The character walks to a desk, sits down, and starts typing when the agent is writing code. It reads when the agent is searching files. It shows a speech bubble when it's waiting for your input.

It's not just eye candy. After using it for a while, you realize there's real utility here. You can glance at the office and immediately know the state of every agent. No need to click through terminal tabs. The visual feedback is instant.

The project also comes with a full layout editor. You can redesign the office, place furniture, paint floors and walls, and export your layout as JSON. It has sound notifications, sub-agent visualization, diverse character sprites. It's a genuinely well-built piece of software.

## Taking It Further: a Standalone App

The original extension works great inside VS Code. But it got me thinking about a different angle. What if this same concept could run as a standalone app, watching *all* Claude Code sessions on the machine at once? Not just the ones launched from VS Code, but sessions running in iTerm, Warp, Cursor, multiple VS Code windows, wherever.

That idea became this fork. Built on top of the original Pixel Agents, this version replaces the VS Code backend with a standalone desktop app using [Tauri](https://tauri.app/) and Rust. The frontend (the pixel art office, the layout editor, all the animations) stays almost untouched. What changed is how the app discovers and monitors Claude Code sessions.

**[GitHub: orseni/pixel-agents](https://github.com/orseni/pixel-agents)**

Instead of managing VS Code terminals, the app watches `~/.claude/projects/` directly. That's the directory where Claude Code stores its JSONL transcript files, regardless of which terminal or editor launched it.

Every five seconds, the Rust backend scans for recently active JSONL files. When it finds one, it creates an agent, starts watching the file for changes, and emits events to the frontend. A new character walks into the office, sits at a desk, and starts doing whatever the real Claude Code session is doing.

When a session goes idle, the character eventually despawns.

No VS Code required. No manual setup. Just open the app and start working with Claude Code however you normally do.

## How It Works Under the Hood

The Rust backend does what the original Node.js/VS Code extension did, but without any editor dependency:

**Session discovery** scans `~/.claude/projects/` for JSONL files that were modified recently. Claude Code organizes transcripts by project, using a hashed version of the directory path as the folder name. The backend reconstructs the original project name from this hash using a greedy path resolution algorithm that checks which directories actually exist on disk.

**File watching** uses polling (1 second interval) to read new lines from each JSONL file incrementally, with partial line buffering for mid-write reads. Same battle-tested approach as the original extension, which learned the hard way that `fs.watch` is unreliable on macOS.

**Transcript parsing** handles all the same JSONL record types: `assistant` messages with tool_use blocks, `user` messages with tool_results, `system` records with turn_duration signals, and `progress` records for sub-agent activity. Each tool gets formatted into a human-readable status ("Reading config.ts", "Running: npm test", "Searching code").

**Timer management** detects when an agent might be stuck waiting for permission (7 second timeout) or has finished a text-only response (5 second idle detection). These are the same heuristics the original uses.

**IPC bridging** is where it gets clever. The frontend was built to receive `MessageEvent`s from VS Code's `postMessage` API. Instead of rewriting all of that, the Tauri adapter listens for Rust backend events and dispatches synthetic `MessageEvent`s on the window object. The React hooks that handle agent state don't know or care that the events are coming from Rust instead of VS Code. Zero changes to the core UI logic.

## What's Different (and What's the Same)

The layout editor works exactly like the original. Floors, walls, furniture, undo/redo, export/import. Your `~/.pixel-agents/layout.json` is shared, so if you used the VS Code extension before, your office layout carries over.

The character animations are identical. Same sprites, same state machine (idle, walk, type, read), same pathfinding, same matrix-style spawn/despawn effects.

What's new:

**Automatic discovery.** You never click "+ Agent". Characters appear and disappear based on what's actually running on your machine.

**Project labels.** Each character shows the name of the project it's working on. When you have agents across multiple projects, this is the thing that makes the office view actually useful.

**One app for everything.** Instead of one extension per VS Code window (each showing only its own terminals), you have one app showing all sessions everywhere.

**Rust backend.** The file watching, JSONL parsing, and timer management run in async Rust with Tokio. It's lightweight and the binary is about 13 MB.

## Running It

You'll need Rust and Node.js installed. Then:

```bash
git clone https://github.com/orseni/pixel-agents.git
cd pixel-agents
npm install
cd webview-ui && npm install && cd ..
npm run dev    # development with hot reload
npm run build  # production binary
```

The production binary ends up at `src-tauri/target/release/pixel-agents`. You can copy it anywhere and run it. The frontend is embedded in the binary.

For day-to-day use, just launch the binary and leave it open. Open Claude Code in any terminal, any project, any editor. The characters show up on their own.

## Credits

All credit for the original concept, the pixel art engine, the layout editor, and the character system goes to [Pablo de Lucca and the Pixel Agents contributors](https://github.com/pablodelucca/pixel-agents). The character sprites are based on the work of [JIK-A-4, Metro City](https://jik-a-4.itch.io/metrocity-free-topdown-character-pack). This fork just takes their excellent work and puts it in a standalone package that can watch your whole machine instead of a single editor window.

If you prefer the VS Code integration, the original extension is great and available on the [VS Code Marketplace](https://marketplace.visualstudio.com/items?itemName=pablodelucca.pixel-agents).

---

*The fork is open source under the MIT license. Issues and contributions are welcome on [GitHub](https://github.com/orseni/pixel-agents).*
