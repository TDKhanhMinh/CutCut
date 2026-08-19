import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';

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
      
      // 2. MOCK: Call to backend to register device & fetch entitlements
      // In a real app: await supabase.functions.invoke('device-entitlement', { body: { installationId, userId } })
      
      console.log(`[Entitlement] Fetching for User ${userId}, Device ${installationId}`);
      
      // Mocking network delay
      await new Promise(resolve => setTimeout(resolve, 500));
      
      // MOCK RESPONSE: Assuming user is PRO for prototype purposes
      const mockResponse = {
        plan: 'PRO' as Plan,
        capabilities: [
          'FEATURE_CLOUD_AI',
          'FEATURE_BATCH_EXPORT',
          'FEATURE_4K_EXPORT'
        ],
        expiresAt: new Date(Date.now() + 30 * 24 * 60 * 60 * 1000).toISOString(),
      };
      
      set({
        plan: mockResponse.plan,
        capabilities: mockResponse.capabilities,
        expiresAt: mockResponse.expiresAt,
        loading: false
      });
      
    } catch (e: any) {
      console.error('Failed to fetch entitlements:', e);
      // Fallback: If network fails, we fall back to FREE, allowing local features to work
      set({
        plan: 'FREE',
        capabilities: [],
        error: e.message || 'Failed to fetch entitlements',
        loading: false
      });
    }
  }
}));
