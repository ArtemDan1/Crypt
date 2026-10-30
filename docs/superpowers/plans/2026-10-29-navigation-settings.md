# Navigation and settings implementation plan

Goal: predictable directory navigation, shadcn autolock selection, confirmed vault removal, seed printing and copying.
Architecture: reuse Vue folder model and shadcn components; enforce destructive authorization in Rust. No git operations. Execute inline as authorized by user.

- [x] Add root directory view with child folders, records, breadcrumbs and creation; retain recent records route and sidebar link. Preserve existing record move controls.
- [x] Replace native autolock select with existing shadcn Select.
- [x] Test vault deletion rejects missing confirmation, locked state and incorrect PIN; successful removal clears directory and quick unlock credentials without touching sibling files. Implement Rust command and confirmation dialog.
- [x] Use Tauri webview print for desktop and browser print for preview, print only seed sheet, add clipboard action with feedback.
- [x] Run frontend tests/build and Rust workspace tests; review resulting changes without git.

Verification: 27 frontend tests passed; all 58 Rust workspace tests passed; production frontend build passed (large-chunk advisory remains). Independent review identified filesystem deletion failure leaving vault live; regression reproduced, fixed, and full suites rerun. Native print dialog and physical printing require manual macOS verification. No git commands used.
