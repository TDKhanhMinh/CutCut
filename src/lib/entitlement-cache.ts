import type { NormalizedEntitlement } from "@/lib/entitlements";

export const ENTITLEMENT_CACHE_TTL_MS = 5 * 60 * 1000;
const CACHE_PREFIX = "cutcut.entitlement-cache.v1.";

interface CachedEntitlement {
  cachedAt: number;
  value: NormalizedEntitlement;
}

function cacheKey(userId: string) {
  return `${CACHE_PREFIX}${userId}`;
}

export function readEntitlementCache(
  userId: string,
  now = Date.now(),
): NormalizedEntitlement | null {
  try {
    const raw = localStorage.getItem(cacheKey(userId));
    if (!raw) return null;
    const cached = JSON.parse(raw) as CachedEntitlement;
    if (!Number.isFinite(cached.cachedAt) || now - cached.cachedAt > ENTITLEMENT_CACHE_TTL_MS) {
      return null;
    }
    if (!cached.value || !Array.isArray(cached.value.capabilities)) return null;
    if (cached.value.expiresAt && Date.parse(cached.value.expiresAt) <= now) return null;
    return {
      plan: cached.value.plan,
      capabilities: cached.value.capabilities.filter((value) => typeof value === "string"),
      expiresAt: cached.value.expiresAt ?? null,
    };
  } catch {
    return null;
  }
}

export function writeEntitlementCache(
  userId: string,
  value: NormalizedEntitlement,
  now = Date.now(),
) {
  try {
    const payload: CachedEntitlement = { cachedAt: now, value };
    localStorage.setItem(cacheKey(userId), JSON.stringify(payload));
  } catch {
    // Cache is an optimization; storage/quota failures must not affect editing.
  }
}

export function clearEntitlementCache(userId: string) {
  try {
    localStorage.removeItem(cacheKey(userId));
  } catch {
    // Best effort only.
  }
}
