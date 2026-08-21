import { createClient } from "@supabase/supabase-js";
import { invoke } from "@tauri-apps/api/core";

const SUPABASE_SESSION_KEY = "supabase-auth-session";

/**
 * Persist Supabase's session in the native OS credential store. The adapter
 * deliberately fails closed: pretending a write succeeded would make the
 * client report a durable session that cannot be restored after restart.
 */
export const tauriSecureStorage = {
  getItem: () =>
    invoke<string | null>("get_secure_token", {
      key: SUPABASE_SESSION_KEY,
    }),
  setItem: (_key: string, value: string) =>
    invoke<void>("set_secure_token", {
      key: SUPABASE_SESSION_KEY,
      value,
    }),
  removeItem: () =>
    invoke<void>("delete_secure_token", {
      key: SUPABASE_SESSION_KEY,
    }),
};

const supabaseUrl = import.meta.env.VITE_SUPABASE_URL || "https://placeholder-project.supabase.co";
const supabaseAnonKey = import.meta.env.VITE_SUPABASE_ANON_KEY || "placeholder-anon-key";

export const supabase = createClient(supabaseUrl, supabaseAnonKey, {
  auth: {
    storage: tauriSecureStorage,
    autoRefreshToken: true,
    persistSession: true,
    detectSessionInUrl: false,
  },
});
