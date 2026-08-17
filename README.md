# CutCut - AI Auto Video Editor

AI Auto Video Editor là một desktop app local-first dành cho việc chỉnh sửa video talking-head/short-form tự động.

## Prerequisites

Để bootstrap dự án, máy của bạn cần cài đặt các dependency sau:

- **OS:** Windows 10/11
- **Node.js:** v18 hoặc mới hơn (khuyên dùng Node 20 LTS)
- **Package Manager:** `npm`
- **Rust Toolchain:** stable (cài đặt qua [rustup](https://rustup.rs/))
- **Tauri Prerequisites:** WebView2 runtime và C++ build tools (Visual Studio 2022 C++ workload) theo [hướng dẫn của Tauri](https://tauri.app/v1/guides/getting-started/prerequisites).

## Setup & Run

1. **Install dependencies:**
   ```bash
   npm install
   ```

2. **Run Development Server (Frontend + Rust Core):**
   ```bash
   npm run tauri dev
   ```

## Repository Structure

- `src/` - Frontend codebase (React + TypeScript + Vite)
  - `components/` - Shared UI components
  - `features/` - Domain-specific feature modules
  - `store/` - Zustand global state
  - `services/` - API/Tauri IPC clients
  - `types/` - Shared TypeScript definitions
- `src-tauri/` - Native Core codebase (Rust + Tauri)
  - `src/commands/` - Tauri IPC command handlers
  - `src/services/` - Rust business logic orchestration
  - `src/models/` - Rust domain models & state
- `docs/` - Kiến trúc và tài liệu dự án

## Engineering Conventions

### Import Aliases
- Sử dụng alias `@/` thay cho relative paths sâu (`../../../`).
- Ví dụ: `import { Button } from "@/components/ui/button"`.

### Naming Conventions
- Component File: `PascalCase.tsx` (ví dụ: `AppLayout.tsx`).
- Hook: `useCamelCase.ts` (ví dụ: `useAppStore.ts`).
- Service/Util: `camelCase.ts` (ví dụ: `mediaService.ts`).

