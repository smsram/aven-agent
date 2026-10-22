# Aven — Desktop AI Agent

Aven is a minimal, Linear-inspired autonomous software engineering desktop application built with **Tauri 2**, **React 19**, **TypeScript**, and **Rust**.

---

## 1. Visual Aesthetics & Design Tokens

| Token | Value | Purpose |
| :--- | :--- | :--- |
| **Main Background** | `#191A1F` | Application workspace canvas |
| **Sidebar** | `#17181D` | Compact navigation rail |
| **Cards & Panels** | `#25262C` | Input surfaces, drawers, and modal panels |
| **Borders** | `#34353C` | Subtle dividers and outline strokes |
| **Primary Accent** | `#7C7CF8` | Electric indigo logo, active nav & cursor |
| **Primary Text** | `#F4F4F5` | Headings, active states, high-contrast text |
| **Secondary Text** | `#A5A6B0` | Body copy, secondary descriptions |
| **Muted Text** | `#6E707B` | Secondary hints and metadata labels |

---

## 2. Refined Window & Navigation Architecture

### Custom Window Title Bar
- **Draggable Region**: `data-tauri-drag-region` enabled with double-click maximize/restore support.
- **Controls**: Native Windows Minimize, Maximize/Restore, and Close buttons on the far right.
- **Single Sidebar Toggle**: Icon switches between `PanelLeftClose` (expanded) and `PanelLeftOpen` (collapsed).
- **Aven Branding**: Uses `public/aven.png` directly as the application logo.
- **Global Search**: Center search field triggering the command palette (`Ctrl+K`).
- *Removed toolbar clutter: workspace labels, Agent Idle pills, Interruption HUD button, and AI Cursor button.*

### Minimal Sidebar Navigation
- **New Chat**: Starts a new task/chat session; visually neutral without persistent highlight.
- **Activity**: Chronological history of tasks and conversations with live status filtering (*All*, *In Progress*, *Completed*, *Failed*, *Paused*).
- **Plugins**: Interface for external Model Context Protocol (MCP) integrations with clean empty state and stdio/SSE configuration modal.
- **Settings**: Subtly separated at the bottom of the sidebar. Opens a dedicated view within the window without persistent sidebar highlight.

---

## 3. Dedicated Settings Structure

1. **General**: Default workspace directory, background running, startup launch, dangerous command confirmations, telemetry.
2. **Appearance**: Linear dark themes (Linear Dark `#191A1F`, Midnight `#0F1014`, Slate `#161922`), density mode, and status bar toggle.
3. **API**: AI model provider configuration (Anthropic, OpenAI, Gemini, Ollama Local, Groq, OpenRouter), model selection, reasoning temperature, and secure Windows Credential Manager storage for API keys with show/hide masking.
4. **Permissions**: Workspace filesystem sandboxing, shell execution policy, and network access rules.
5. **Memory**: Persistent agent memory, local vector index, and context window slider.
6. **Shortcuts**: Interactive shortcut guide (`Ctrl+1..3`, `Ctrl+B`, `Ctrl+K`, `Ctrl+,`).

---

## 4. Launch & Build Commands

```bash
# 1. Start browser development preview (Vite dev server)
npm run dev

# 2. Run as native Windows desktop application with Tauri 2
npm run tauri:dev

# 3. Build production Windows executable and installer (.msi / .exe)
npm run tauri:build
```
