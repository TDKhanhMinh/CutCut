import "jsr:@supabase/functions-js/edge-runtime.d.ts";
import { createClient } from "jsr:@supabase/supabase-js@2";

const MAX_BODY_BYTES = 16 * 1024;
const DEVICE_HASH = /^[0-9a-f]{64}$/;
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
const SAFE_LABEL = /^[A-Za-z0-9 ._-]{1,64}$/;
const SAFE_VERSION = /^[A-Za-z0-9._+-]{1,32}$/;
const SAFE_PLATFORM = /^[A-Za-z0-9._-]{1,16}$/;

function allowedOrigin(req: Request): string {
  const configured = (Deno.env.get("ALLOWED_ORIGINS") ?? "tauri://localhost,http://localhost:1420")
    .split(",")
    .map((value) => value.trim())
    .filter(Boolean);
  const origin = req.headers.get("Origin") ?? "";
  return configured.includes(origin) ? origin : (configured[0] ?? "tauri://localhost");
}

function reply(body: unknown, status: number, origin: string) {
  return new Response(JSON.stringify(body), {
    status,
    headers: {
      "Access-Control-Allow-Origin": origin,
      "Access-Control-Allow-Headers": "authorization, x-client-info, apikey, content-type",
      "Access-Control-Allow-Methods": "POST, OPTIONS",
      "Content-Type": "application/json",
      "Cache-Control": "no-store",
    },
  });
}

function asObject(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error("invalid payload");
  return value as Record<string, unknown>;
}

function requiredString(value: unknown, pattern: RegExp, message: string): string {
  if (typeof value !== "string" || !pattern.test(value)) throw new Error(message);
  return value;
}

function deviceLimit(record: Record<string, unknown> | null): number {
  const plan = typeof record?.plan_id === "string" ? record.plan_id.toUpperCase() : "FREE";
  const features = record?.features;
  const configured = features && typeof features === "object" && !Array.isArray(features)
    ? (features as Record<string, unknown>).maxDevices
    : undefined;
  if (typeof configured === "number" && Number.isInteger(configured) && configured >= 1 && configured <= 64) {
    return configured;
  }
  if (plan === "ENTERPRISE") return 10;
  if (plan === "PRO") return 3;
  return 1;
}

async function authenticate(req: Request) {
  const authHeader = req.headers.get("Authorization") ?? "";
  if (!/^Bearer\s+[^\s]+$/i.test(authHeader)) return null;
  const url = Deno.env.get("SUPABASE_URL") ?? "";
  const anonKey = Deno.env.get("SUPABASE_ANON_KEY") ?? "";
  const client = createClient(url, anonKey, { global: { headers: { Authorization: authHeader } } });
  const { data: { user }, error } = await client.auth.getUser();
  return error || !user ? null : { user, authHeader };
}

Deno.serve(async (req) => {
  const origin = allowedOrigin(req);
  if (req.method === "OPTIONS") return reply({ ok: true }, 200, origin);
  if (req.method !== "POST") return reply({ error: "method_not_allowed" }, 405, origin);

  const auth = await authenticate(req);
  if (!auth) return reply({ error: "unauthorized" }, 401, origin);

  const serviceKey = Deno.env.get("SUPABASE_SERVICE_ROLE_KEY") ?? "";
  const supabaseUrl = Deno.env.get("SUPABASE_URL") ?? "";
  if (!serviceKey || !supabaseUrl) return reply({ error: "service_unavailable" }, 503, origin);

  try {
    const declaredLength = Number(req.headers.get("Content-Length") ?? "0");
    if (declaredLength > MAX_BODY_BYTES) return reply({ error: "payload_too_large" }, 413, origin);
    const raw = await req.text();
    if (new TextEncoder().encode(raw).length > MAX_BODY_BYTES) {
      return reply({ error: "payload_too_large" }, 413, origin);
    }
    const body = asObject(JSON.parse(raw));
    const action = body.action;
    if (action !== "activate" && action !== "deactivate" && action !== "status") {
      return reply({ error: "invalid_action" }, 400, origin);
    }

    const admin = createClient(supabaseUrl, serviceKey);
    if (action === "activate") {
      const deviceHash = requiredString(body.deviceHash, DEVICE_HASH, "invalid_device_hash");
      const deviceLabel = requiredString(body.deviceLabel, SAFE_LABEL, "invalid_device_label");
      const appVersion = requiredString(body.appVersion, SAFE_VERSION, "invalid_app_version");
      const platform = requiredString(body.platform, SAFE_PLATFORM, "invalid_platform");
      const { data: entitlement, error: entitlementError } = await admin
        .from("entitlements")
        .select("plan_id, features, expires_at")
        .eq("user_id", auth.user.id)
        .order("created_at", { ascending: false })
        .limit(1)
        .maybeSingle();
      if (entitlementError) return reply({ error: "service_unavailable" }, 503, origin);
      const expiresAt = typeof entitlement?.expires_at === "string" ? Date.parse(entitlement.expires_at) : NaN;
      const activeEntitlement = !Number.isFinite(expiresAt) || expiresAt > Date.now();
      const limit = activeEntitlement ? deviceLimit(entitlement) : 1;
      const { data: deviceId, error } = await admin.rpc("activate_device", {
        p_user_id: auth.user.id,
        p_device_hash: deviceHash,
        p_device_label: deviceLabel,
        p_app_version: appVersion,
        p_platform: platform,
        p_device_limit: limit,
      });
      if (error) {
        if (error.message?.includes("device_limit_exceeded")) {
          return reply({ error: "device_limit_exceeded" }, 409, origin);
        }
        return reply({ error: "service_unavailable" }, 503, origin);
      }
      return reply({ activated: true, deviceId, deviceLimit: limit }, 200, origin);
    }

    if (action === "deactivate") {
      const deviceId = requiredString(body.deviceId, UUID, "invalid_device_id");
      const { data: deactivated, error } = await admin.rpc("deactivate_device", {
        p_user_id: auth.user.id,
        p_device_id: deviceId,
      });
      if (error) return reply({ error: "service_unavailable" }, 503, origin);
      return reply({ deactivated: deactivated === true }, 200, origin);
    }

    const { data: devices, error } = await admin.rpc("list_user_devices", { p_user_id: auth.user.id });
    if (error) return reply({ error: "service_unavailable" }, 503, origin);
    return reply({ devices: Array.isArray(devices) ? devices : [] }, 200, origin);
  } catch {
    return reply({ error: "invalid_request" }, 400, origin);
  }
});
