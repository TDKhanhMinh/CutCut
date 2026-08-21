import "jsr:@supabase/functions-js/edge-runtime.d.ts";
import { createClient } from "jsr:@supabase/supabase-js@2";

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
      "Access-Control-Allow-Methods": "GET, POST, OPTIONS",
      "Content-Type": "application/json",
      "Cache-Control": "no-store",
    },
  });
}

Deno.serve(async (req) => {
  const origin = allowedOrigin(req);
  if (req.method === "OPTIONS") return reply({ ok: true }, 200, origin);
  if (req.method !== "GET" && req.method !== "POST") {
    return reply({ error: "method_not_allowed" }, 405, origin);
  }

  const authHeader = req.headers.get("Authorization") ?? "";
  if (!/^Bearer\s+[^\s]+$/i.test(authHeader)) {
    return reply({ error: "unauthorized" }, 401, origin);
  }

  try {
    const supabaseUrl = Deno.env.get("SUPABASE_URL") ?? "";
    const anonKey = Deno.env.get("SUPABASE_ANON_KEY") ?? "";
    const serviceKey = Deno.env.get("SUPABASE_SERVICE_ROLE_KEY") ?? "";
    if (!supabaseUrl || !serviceKey) return reply({ error: "service_unavailable" }, 503, origin);

    const userClient = createClient(supabaseUrl, anonKey, {
      global: { headers: { Authorization: authHeader } },
    });
    const {
      data: { user },
      error: authError,
    } = await userClient.auth.getUser();
    if (authError || !user) return reply({ error: "unauthorized" }, 401, origin);

    const admin = createClient(supabaseUrl, serviceKey);
    const { data, error } = await admin.rpc("get_ai_quota_status", { p_user_id: user.id });
    if (error) return reply({ error: "service_unavailable" }, 503, origin);
    const row = Array.isArray(data) ? data[0] : data;
    return reply(
      {
        requestsUsed: Number(row?.requests_used ?? 0),
        requestLimit: Number(row?.request_limit ?? 20),
        requestsRemaining: Number(row?.requests_remaining ?? 0),
        windowUsed: Number(row?.window_used ?? 0),
        windowLimit: Number(row?.window_limit ?? 5),
        windowRemaining: Number(row?.window_remaining ?? 0),
        entitlementActive: row?.entitlement_active === true,
        entitlementExpiresAt:
          typeof row?.entitlement_expires_at === "string" ? row.entitlement_expires_at : null,
      },
      200,
      origin,
    );
  } catch {
    return reply({ error: "service_unavailable" }, 503, origin);
  }
});
