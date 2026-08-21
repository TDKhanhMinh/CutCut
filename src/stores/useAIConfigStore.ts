import { create } from "zustand";
import {
  deleteGeminiApiKey,
  getGeminiKeyStatus,
  setGeminiApiKey,
  testGeminiKey,
  type GeminiKeyStatus,
} from "@/services/gemini";

export type AIMode = "hosted" | "byok";

interface AIConfigState {
  mode: AIMode;
  keyStatus: GeminiKeyStatus;
  loading: boolean;
  error: string | null;
  hydrate: () => Promise<void>;
  setMode: (mode: AIMode) => void;
  saveKey: (apiKey: string) => Promise<void>;
  removeKey: () => Promise<void>;
  testKey: () => Promise<void>;
}

const readMode = (): AIMode => {
  if (typeof window === "undefined") return "hosted";
  return window.localStorage.getItem("cutcut-ai-mode") === "byok" ? "byok" : "hosted";
};

export const useAIConfigStore = create<AIConfigState>((set) => ({
  mode: readMode(),
  keyStatus: { configured: false, maskedHint: null },
  loading: false,
  error: null,

  hydrate: async () => {
    set({ loading: true, error: null });
    try {
      set({ keyStatus: await getGeminiKeyStatus(), loading: false });
    } catch (cause) {
      set({
        loading: false,
        error: cause instanceof Error ? cause.message : String(cause),
      });
    }
  },

  setMode: (mode) => {
    if (typeof window !== "undefined") window.localStorage.setItem("cutcut-ai-mode", mode);
    set({ mode, error: null });
  },

  saveKey: async (apiKey) => {
    set({ loading: true, error: null });
    try {
      set({ keyStatus: await setGeminiApiKey(apiKey), mode: "byok", loading: false });
      if (typeof window !== "undefined") window.localStorage.setItem("cutcut-ai-mode", "byok");
    } catch (cause) {
      set({
        loading: false,
        error: cause instanceof Error ? cause.message : String(cause),
      });
      throw cause;
    }
  },

  removeKey: async () => {
    set({ loading: true, error: null });
    try {
      await deleteGeminiApiKey();
      set({ keyStatus: { configured: false, maskedHint: null }, mode: "hosted", loading: false });
      if (typeof window !== "undefined") window.localStorage.setItem("cutcut-ai-mode", "hosted");
    } catch (cause) {
      set({
        loading: false,
        error: cause instanceof Error ? cause.message : String(cause),
      });
      throw cause;
    }
  },

  testKey: async () => {
    set({ loading: true, error: null });
    try {
      await testGeminiKey();
      set({ loading: false });
    } catch (cause) {
      set({
        loading: false,
        error: cause instanceof Error ? cause.message : String(cause),
      });
      throw cause;
    }
  },
}));
