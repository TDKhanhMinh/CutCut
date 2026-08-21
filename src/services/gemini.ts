import { invoke } from "@tauri-apps/api/core";

export interface GeminiKeyStatus {
  configured: boolean;
  maskedHint: string | null;
}

export const getGeminiKeyStatus = () => invoke<GeminiKeyStatus>("get_gemini_key_status");

export const setGeminiApiKey = (apiKey: string) =>
  invoke<GeminiKeyStatus>("set_gemini_api_key", { apiKey });

export const deleteGeminiApiKey = () => invoke<void>("delete_gemini_api_key");

export const testGeminiKey = () => invoke<void>("test_gemini_key");
