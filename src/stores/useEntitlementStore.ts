import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import { supabase } from '../lib/supabase';

export type Plan = 'FREE' | 'PRO' | 'ENTERPRISE';

export interface EntitlementState {
  plan: Plan;
  capabilities: string[];
  expiresAt: string | null;
  loading: boolean;
  error: string | null;
  
  // Actions
  fetchEntitlements: (userId: string) => Promise<void>;
  clearEntitlements: () => void;
  hasCapability: (cap: string) => boolean;
}

export const useEntitlementStore = create<EntitlementState>((set, get) => ({
  plan: 'FREE',
  capabilities: [],
  expiresAt: null,
  loading: false,
  error: null,

  hasCapability: (cap: string) => {
    return get().capabilities.includes(cap);
  },

  clearEntitlements: () => {
    set({
      plan: 'FREE',
      capabilities: [],
      expiresAt: null,
      error: null,
    });
  },

  fetchEntitlements: async (userId: string) => {
    set({ loading: true, error: null });
    try {
      // 1. Get or create a random UUIDv4 for this installation via Native Rust
      const installationId = await invoke<string>('get_or_create_installation_id');
      
      // 2. Register device on server (best-effort, do not block entitlement fetch)
      // In production: await supabase.functions.invoke('register-device', { body: { installationId, userId } })
      void installationId; // suppress unused warning until Edge Function is ready
      
      // 3. Fetch entitlement from server — do NOT mock
      const { data, error } = await supabase
        .from('entitlements')
        .select('plan, capabilities, expires_at')
        .eq('user_id', userId)
        .order('created_at', { ascending: false })
        .limit(1)
        .maybeSingle();

      if (error) throw error;

      const plan = (data?.plan ?? 'FREE') as Plan;
      const capabilities: string[] = data?.capabilities ?? [];
      const expiresAt: string | null = data?.expires_at ?? null;

      set({ plan, capabilities, expiresAt, loading: false });

    } catch (e: unknown) {
      console.error('Failed to fetch entitlements:', e);
      // Offline fallback: FREE plan, local features remain available
      set({
        plan: 'FREE',
        capabilities: [],
        error: e instanceof Error ? e.message : 'Failed to fetch entitlements',
        loading: false,
      });
    }
  }
}));
