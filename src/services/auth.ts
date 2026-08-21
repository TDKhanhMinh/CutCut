import { invoke } from "@tauri-apps/api/core";

export interface AuthSession {
  accessToken: string;
  expiresAt: number | null;
  userId: string | null;
}

/** Keep Supabase tokens in the native process only; never persist them in a project file. */
export const setAuthSession = (session: AuthSession) =>
  invoke<void>("set_auth_session", {
    accessToken: session.accessToken,
    expiresAt: session.expiresAt,
    userId: session.userId,
  });

export const getAuthSession = () => invoke<AuthSession | null>("get_auth_session");

export const clearAuthSession = () => invoke<void>("clear_auth_session");
