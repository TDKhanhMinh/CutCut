import { createClient } from "@supabase/supabase-js";
import { invoke } from "@tauri-apps/api/core";

const tauriSecureStorage = {
  // Supabase may provide a project-derived storage key. Map it to one
  // allowlisted native credential name so arbitrary keyring entries cannot be
  // addressed through the generic storage adapter.
  storageKey: "supabase-auth-session",
  getItem: async (key: string): Promise<string | null> => {
    try {
      void key;
      const val = await invoke<string | null>("get_secure_token", {
        key: tauriSecureStorage.storageKey,
      });
      return val;
    } catch (e) {
      console.warn("Failed to read from secure storage:", e);
      return null;
    }
  },
  setItem: async (key: string, value: string): Promise<void> => {
    try {
      void key;
      await invoke("set_secure_token", { key: tauriSecureStorage.storageKey, value });
    } catch (e) {
      console.warn("Failed to write to secure storage:", e);
    }
  },
  removeItem: async (key: string): Promise<void> => {
    try {
      void key;
      await invoke("delete_secure_token", { key: tauriSecureStorage.storageKey });
    } catch (e) {
      console.warn("Failed to remove from secure storage:", e);
    }
  },
};

// Ensure you replace these with actual environment variables or configuration values
// For this prototype, we'll use a placeholder. In reality, these come from Vite env
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
