# Markdown editor

Approved direction: replace the limited CodeMirror live preview with Milkdown Crepe, as agreed in conversation. Keep Markdown as the storage format and the existing debounced saving, record history and encryption unchanged.

Crepe provides formatting, headings, nested lists, tasks, tables, code blocks with language selection and syntax highlighting, slash commands and undo. Adapt its light/dark appearance to Crypt. Bundle dependencies locally; the editor must work offline.

Preserve Crypt's ULID wikilinks and autocomplete/create-record workflow. A custom inline schema must parse and serialize `[[id|alias]]` without escaping or converting links. Code examples containing this text must remain code. Opening a note must not save or normalize it; only actual edits update the model. Keep source Markdown accessible for unsupported constructs and exact editing.

Approaches considered: extend the existing CodeMirror implementation (more maintenance), Vditor (multiple Markdown modes), Milkdown Crepe (selected by user, ready-made editing UI and Vue-compatible lifecycle). All require app-specific link integration. This is an editor replacement, not a clone of Obsidian's plugins or vault system.

Verification: existing frontend suite, real parser/serializer round trips, editor integration and lifecycle checks, production frontend build and Tauri app build. Install the built app over /Applications/Crypt.app, preserving a backup of the previous app. Do not touch vault data.
