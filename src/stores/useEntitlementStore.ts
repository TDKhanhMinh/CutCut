import { create } from "zustand";
import { supabase } from "@/lib/supabase";
import {
  clearEntitlementCache,
  readEntitlementCache,
  writeEntitlementCache,
} from "@/lib/entitlement-cache";
import { normalizeEntitlement, type NormalizedEntitlement } from "@/lib/entitlements";
import { deviceService } from "@/services/device";

export type Plan = "FREE" | "PRO" | "ENTERPRISE";

export interface EntitlementState {
  userId: string | null;
  plan: Plan;
  capabilities: string[];
  expiresAt: string | null;
  loading: boolean;
  error: string | null;
  lastFetchedAt: number | null;
  fetchEntitlements: (userId: string) => Promise<void>;
  refreshIfStale: (userId: string) => Promise<void>;
  clearEntitlements: () => void;
  hasCapability: (cap: string) => boolean;
}

let activeFetch: { userId: string; promise: Promise<void> } | null = null;
const CACHE_TTL_MS = 5 * 60 * 1000;

function applyNormalized(
  set: (state: Partial<EntitlementState>) => void,
  userId: string,
  value: NormalizedEntitlement,
  fetchedAt: number,
) {
  set({
    userId,
    plan: value.plan,
    capabilities: value.capabilities,
    expiresAt: value.expiresAt,
    lastFetchedAt: fetchedAt,
    loading: false,
    error: null,
  });
}

export const useEntitlementStore = create<EntitlementState>((set, get) => ({
  userId: null,
  plan: "FREE",
  capabilities: [],
  expiresAt: null,
  loading: false,
  error: null,
  lastFetchedAt: null,

  hasCapability: (cap) => get().capabilities.includes(cap),

  clearEntitlements: () => {
    const previousUserId = get().userId;
    if (previousUserId) clearEntitlementCache(previousUserId);
    set({
      userId: null,
      plan: "FREE",
      capabilities: [],
      expiresAt: null,
      loading: false,
      error: null,
      lastFetchedAt: null,
    });
  },

  fetchEntitlements: async (userId) => {
    if (activeFetch?.userId === userId) return activeFetch.promise;

    const promise = (async () => {
      const startedAt = Date.now();
      set({ userId, loading: true, error: null });
      try {
        // The Edge Function is idempotent for this installation and owns the
        // server-side device limit. It never receives a raw hardware ID.
        await deviceService.activate();

        const { data, error } = await supabase
          .from("entitlements")
          .select("plan_id, features, expires_at")
          .eq("user_id", userId)
          .order("created_at", { ascending: false })
          .limit(1)
          .maybeSingle();
        if (error) throw error;

        const normalized = normalizeEntitlement(data);
        writeEntitlementCache(userId, normalized, startedAt);
        applyNormalized(set, userId, normalized, startedAt);
      } catch (error) {
        const cached = readEntitlementCache(userId);
        if (cached) {
          applyNormalized(set, userId, cached, get().lastFetchedAt ?? startedAt);
          set({ error: "entitlement_offline" });
        } else {
          set({
            userId,
            plan: "FREE",
            capabilities: [],
            expiresAt: null,
            loading: false,
            error: "entitlement_unavailable",
            lastFetchedAt: null,
          });
        }
        // Do not log the remote error: auth/provider details must not leak into
        // desktop logs. The server remains the final authorization boundary.
        void error;
      }
    })();

    activeFetch = { userId, promise };
    try {
      await promise;
    } finally {
      if (activeFetch?.promise === promise) activeFetch = null;
    }
  },

  refreshIfStale: async (userId) => {
    const lastFetchedAt = get().userId === userId ? get().lastFetchedAt : null;
    if (lastFetchedAt && Date.now() - lastFetchedAt < CACHE_TTL_MS) return;
    await get().fetchEntitlements(userId);
  },
}));
