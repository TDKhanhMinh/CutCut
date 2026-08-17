# Architecture Boundaries

This document defines the ownership rules and module boundaries for the CutCut Video Editor. It ensures the UI, native layer, and media processing components remain decoupled as the project scales.

## 1. Frontend Architecture (React / TypeScript)

- **`src/features/`**: Contains domain-specific UI features (e.g., Timeline, VideoPlayer, TranscriptEditor). Each feature should be self-contained and expose only its root components.
- **`src/store/`**: Global state management using Zustand. State should be organized by domain (e.g., `useTimelineStore`, `useProjectStore`). Features should subscribe to these stores rather than passing props deeply.
- **`src/services/`**: Frontend-side API abstractions. This layer is responsible for invoking Tauri commands. UI components MUST NOT call `invoke` directly. They should call functions from `src/services/`.
- **`src/types/`**: Canonical domain models and TypeScript interfaces shared across the frontend. These types must map to the Rust models.
- **`src/components/ui/`**: Reusable, dumb UI components (shadcn/ui).
- **`src/components/layout/`**: Application shell layout components (Sidebar, Header, Workspace).

## 2. Backend Architecture (Rust / Tauri Core)

- **`src-tauri/src/commands/`**: Tauri IPC command handlers. These functions should be thin wrappers that deserialize arguments, call the appropriate `services`, and serialize the result. They DO NOT contain business logic.
- **`src-tauri/src/services/`**: Application services. This layer orchestrates business logic, state management, and file system interactions. It acts as the bridge between `commands` and `engines`.
- **`src-tauri/src/engines/`**: Core media processing and AI adapters. This layer wraps external dependencies like FFmpeg, Whisper (local STT), and VAD engines. It provides a canonical Rust interface for the `services` layer to use.
- **`src-tauri/src/models/`**: Canonical Rust structs representing the domain (e.g., `Project`, `EditAction`, `Transcript`). These structs derive `Serialize` and `Deserialize` to communicate with the Frontend.

## 3. Communication & Rules

- **Strict IPC Boundary**: The frontend can only communicate with the native layer via Tauri commands defined in `src-tauri/src/commands/`.
- **No Direct Process Spawning in UI**: The React frontend cannot spawn child processes. All media operations must be delegated to the Rust `engines` via `services`.
- **Typed Contracts**: Any data passed across the IPC boundary must be strongly typed in both `src/types/` (TypeScript) and `src-tauri/src/models/` (Rust).
