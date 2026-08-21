export type EntitlementPlan = 'FREE' | 'PRO' | 'ENTERPRISE';

export interface EntitlementRecord {
  plan_id?: unknown;
  features?: unknown;
  expires_at?: unknown;
}

export interface NormalizedEntitlement {
  plan: EntitlementPlan;
  capabilities: string[];
  expiresAt: string | null;
}

/**
 * Convert the cloud schema's plan_id/features JSON into the UI contract.
 * The parser is deliberately conservative: malformed remote data must not
 * unlock a capability or make offline local editing unavailable.
 */
export function normalizeEntitlement(
  record: EntitlementRecord | null | undefined,
): NormalizedEntitlement {
  const rawPlan = typeof record?.plan_id === 'string' ? record.plan_id.toUpperCase() : 'FREE';
  const plan: EntitlementPlan =
    rawPlan === 'PRO' || rawPlan === 'ENTERPRISE' ? rawPlan : 'FREE';

  const capabilities = new Set<string>();
  const features = record?.features;
  if (Array.isArray(features)) {
    for (const feature of features) {
      if (typeof feature === 'string' && feature.length > 0) capabilities.add(feature);
    }
  } else if (features && typeof features === 'object') {
    const rawCapabilities = (features as { capabilities?: unknown }).capabilities;
    if (Array.isArray(rawCapabilities)) {
      for (const feature of rawCapabilities) {
        if (typeof feature === 'string' && feature.length > 0) capabilities.add(feature);
      }
    } else {
      for (const [feature, enabled] of Object.entries(features)) {
        if (enabled === true) capabilities.add(feature);
      }
    }
  }

  const expiresAt = typeof record?.expires_at === 'string' ? record.expires_at : null;
  return { plan, capabilities: [...capabilities], expiresAt };
}
