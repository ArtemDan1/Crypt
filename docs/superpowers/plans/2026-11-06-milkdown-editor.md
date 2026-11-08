# Milkdown Editor Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Ship the approved Crepe editor in Crypt and replace the installed macOS app.

**Architecture:** Vue hosts Crepe and synchronizes actual document edits with the existing Markdown model. A Milkdown inline-node plugin preserves Crypt wikilinks. Keep a source mode for exact Markdown editing.

**Tech Stack:** Vue 3, Milkdown Crepe/kit, CodeMirror source mode, Tauri 2.

**Spec:** docs/superpowers/specs/2026-11-06-milkdown-editor-design.md

## Global Constraints

- Markdown remains the storage format; opening notes must not trigger normalization saves.
- Keep existing debounce, history and encryption.
- Bundle assets locally and support both application themes.

## Review Focus

- Existing links and link-shaped code survive serialization.
- Switching notes during asynchronous initialization cannot leak document content.
- Leaving the editor immediately after typing saves the latest edit.
- Source mode and external history restoration synchronize without feedback loops.
- Empty notes, tables and fenced code remain editable offline.

### Task 1: Dependencies and Markdown compatibility

Files: package.json, pnpm-lock.yaml, components/editor/milkdownLinks.ts, tests/milkdown.test.ts.

- [x] Install Crepe/kit and inspect installed API and source.
- [x] Write and run failing parser/serializer tests for ULID links, aliases, links in code and tables.
- [x] Implement the inline-node plugin with input autocomplete and link navigation.
- [x] Verify round trips and existing suite.

### Task 2: Vue editor integration

Files: components/editor/MarkdownEditor.vue, components/editor/milkdown.css, tests/markdownEditor.test.ts.

- [x] Write failing integration tests for initialization, edit synchronization, source switching and teardown.
- [x] Replace host implementation with Crepe; add source-mode fallback and localized UI.
- [x] Match Crypt typography, colors and width constraints. Ensure assets are bundled.
- [x] Verify tests, frontend build and review the complete diff.

### Task 3: macOS delivery

Files: app version metadata and lockfiles.

- [x] Set version to 2026.11.6; build Tauri .app.
- [x] Verify app signature and bundle metadata.
- [x] Back up previous installed app, close it gracefully, install and verify binary hashes.
- [x] Commit the completed changes on feature/milkdown-editor; do not merge or push.
