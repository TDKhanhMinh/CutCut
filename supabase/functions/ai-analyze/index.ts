import "jsr:@supabase/functions-js/edge-runtime.d.ts";
import { createClient } from "jsr:@supabase/supabase-js@2";
import { SEMANTIC_PROMPT_VERSION, SEMANTIC_SYSTEM_PROMPT } from "./prompt.ts";

const MAX_BODY_BYTES = 256 * 1024;
const MAX_SEGMENTS = 256;
const MAX_SEGMENT_TEXT = 2_000;
const MAX_TOTAL_TEXT = 100_000;
const MAX_INSTRUCTIONS = 4_000;
const MAX_ACTIONS = 256;
const PROVIDER_TIMEOUT_MS = 30_000;

function response(body: unknown, status: number, origin: string) {
  return new Response(JSON.stringify(body), {
    status,
    headers: {
      "Access-Control-Allow-Origin": origin,
      "Access-Control-Allow-Headers":
        "authorization, x-client-info, apikey, content-type, x-request-id",
      "Access-Control-Allow-Methods": "POST, OPTIONS",
      "Content-Type": "application/json",
      "Cache-Control": "no-store",
    },
  });
}

function allowedOrigin(req: Request): string {
  const configured = (Deno.env.get("ALLOWED_ORIGINS") ?? "tauri://localhost,http://localhost:1420")
    .split(",")
    .map((value) => value.trim())
    .filter(Boolean);
  const origin = req.headers.get("Origin") ?? "";
  return configured.includes(origin) ? origin : (configured[0] ?? "tauri://localhost");
}

function isFiniteInteger(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value) && Number.isInteger(value);
}

function validRequestId(value: unknown): value is string {
  return typeof value === "string" && /^[A-Za-z0-9_-]{8,128}$/.test(value);
}

function validatePayload(payload: unknown) {
  if (!payload || typeof payload !== "object") throw new Error("invalid payload");
  const body = payload as Record<string, unknown>;
  if (!validRequestId(body.requestId)) throw new Error("requestId is required");
  if (
    typeof body.sourceMediaId !== "string" ||
    !/^[A-Za-z0-9._-]{1,128}$/.test(body.sourceMediaId)
  ) {
    throw new Error("sourceMediaId is required");
  }
  if (
    !Array.isArray(body.segments) ||
    body.segments.length === 0 ||
    body.segments.length > MAX_SEGMENTS
  ) {
    throw new Error("segments must contain 1-256 items");
  }
  let totalChars = 0;
  const seen = new Set<string>();
  const segments = body.segments.map((candidate) => {
    if (!candidate || typeof candidate !== "object") throw new Error("invalid segment");
    const segment = candidate as Record<string, unknown>;
    if (
      typeof segment.id !== "string" ||
      !/^[A-Za-z0-9._-]{1,128}$/.test(segment.id) ||
      seen.has(segment.id)
    ) {
      throw new Error("segment ids must be unique and safe");
    }
    if (
      !isFiniteInteger(segment.startMs) ||
      !isFiniteInteger(segment.endMs) ||
      segment.startMs < 0 ||
      segment.endMs <= segment.startMs ||
      segment.endMs > 86_400_000
    ) {
      throw new Error("segment timestamps must be valid milliseconds");
    }
    if (typeof segment.text !== "string" || segment.text.length > MAX_SEGMENT_TEXT)
      throw new Error("segment text is too long");
    seen.add(segment.id);
    totalChars += segment.text.length;
    return { id: segment.id, startMs: segment.startMs, endMs: segment.endMs, text: segment.text };
  });
  if (totalChars > MAX_TOTAL_TEXT) throw new Error("transcript is too large");
  if (
    body.instructions !== undefined &&
    (typeof body.instructions !== "string" || body.instructions.length > MAX_INSTRUCTIONS)
  ) {
    throw new Error("instructions are too long");
  }
  return {
    requestId: body.requestId as string,
    sourceMediaId: body.sourceMediaId as string,
    segments,
    instructions: typeof body.instructions === "string" ? body.instructions : "",
    config: (body.config && typeof body.config === "object" ? body.config : {}) as Record<
      string,
      unknown
    >,
  };
}

function validateActions(raw: unknown, input: ReturnType<typeof validatePayload>) {
  if (!Array.isArray(raw) || raw.length > MAX_ACTIONS)
    throw new Error("provider output action list is invalid");
  const boundaries = new Map(
    input.segments.map((segment) => [`${segment.startMs}:${segment.endMs}`, segment.id]),
  );
  const ids = new Set<string>();
  return raw.map((candidate, index) => {
    if (!candidate || typeof candidate !== "object")
      throw new Error("provider output action is invalid");
    const action = candidate as Record<string, unknown>;
    const startMs = action.startMs;
    const endMs = action.endMs;
    if (
      !isFiniteInteger(startMs) ||
      !isFiniteInteger(endMs) ||
      !boundaries.has(`${startMs}:${endMs}`)
    ) {
      throw new Error("provider output timestamp is not an input boundary");
    }
    const actionType = typeof action.action === "string" ? action.action.toUpperCase() : "";
    if (!["CUT", "KEEP", "HIGHLIGHT"].includes(actionType))
      throw new Error("provider output action type is invalid");
    const confidence = typeof action.confidence === "number" ? action.confidence : NaN;
    if (!Number.isFinite(confidence) || confidence < 0 || confidence > 1)
      throw new Error("provider output confidence is invalid");
    if (actionType === "CUT" && confidence < 0.8)
      throw new Error("provider output cut confidence is below the conservative threshold");
    const taxonomy = typeof action.taxonomy === "string" ? action.taxonomy : "none";
    if (
      ![
        "false_start",
        "repeated_take",
        "redundant_sentence",
        "important_statement",
        "none",
      ].includes(taxonomy)
    )
      throw new Error("provider output taxonomy is invalid");
    const id =
      typeof action.id === "string" && /^[A-Za-z0-9._-]{1,128}$/.test(action.id)
        ? action.id
        : `ai-${index + 1}`;
    if (ids.has(id)) throw new Error("provider output ids must be unique");
    ids.add(id);
    const reason = typeof action.reason === "string" ? action.reason.slice(0, 500) : "";
    return {
      id,
      sourceMediaId: input.sourceMediaId,
      startMs,
      endMs,
      action: actionType,
      reason,
      confidence,
      taxonomy,
      source: "ai",
      segmentIds: [boundaries.get(`${startMs}:${endMs}`)!],
    };
  });
}

Deno.serve(async (req) => {
  const origin = allowedOrigin(req);
  if (req.method === "OPTIONS") return response({ ok: true }, 200, origin);
  if (req.method !== "POST") return response({ error: "method_not_allowed" }, 405, origin);

  const authHeader = req.headers.get("Authorization") ?? "";
  if (!/^Bearer\s+[^\s]+$/i.test(authHeader))
    return response({ error: "unauthorized" }, 401, origin);
  const declaredLength = Number(req.headers.get("Content-Length") ?? "0");
  if (declaredLength > MAX_BODY_BYTES) return response({ error: "payload_too_large" }, 413, origin);

  let requestId = "unknown";
  try {
    const supabaseUrl = Deno.env.get("SUPABASE_URL") ?? "";
    const anonKey = Deno.env.get("SUPABASE_ANON_KEY") ?? "";
    const serviceKey = Deno.env.get("SUPABASE_SERVICE_ROLE_KEY") ?? "";
    const userClient = createClient(supabaseUrl, anonKey, {
      global: { headers: { Authorization: authHeader } },
    });
    const {
      data: { user },
      error: authError,
    } = await userClient.auth.getUser();
    if (authError || !user) return response({ error: "unauthorized" }, 401, origin);

    const rawBody = await req.text();
    if (new TextEncoder().encode(rawBody).length > MAX_BODY_BYTES)
      return response({ error: "payload_too_large" }, 413, origin);
    const payload = validatePayload(JSON.parse(rawBody));
    requestId = payload.requestId;
    if (!serviceKey) return response({ error: "service_unavailable" }, 503, origin);
    const admin = createClient(supabaseUrl, serviceKey);

    const { data: previous } = await admin
      .from("ai_usage")
      .select("metadata")
      .eq("user_id", user.id)
      .eq("request_id", requestId)
      .maybeSingle();
    const cachedResponse = previous?.metadata?.response;
    if (cachedResponse && typeof cachedResponse === "object")
      return response(cachedResponse, 200, origin);

    const { data: quotaAvailable, error: quotaCheckError } = await admin.rpc("check_ai_quota", {
      p_user_id: user.id,
    });
    if (quotaCheckError) {
      console.error("quota check failed", { requestId });
      return response({ error: "service_unavailable" }, 503, origin);
    }
    if (!quotaAvailable) return response({ error: "quota_exceeded" }, 429, origin);

    const apiKey = Deno.env.get("GEMINI_API_KEY");
    if (!apiKey) return response({ error: "service_unavailable" }, 503, origin);
    const model = Deno.env.get("GEMINI_MODEL") ?? "gemini-1.5-flash";
    const transcript = payload.segments
      .map((segment) => `[${segment.startMs}-${segment.endMs}] ${segment.id}: ${segment.text}`)
      .join("\n");
    const prompt = `${transcript}${payload.instructions ? `\n\nUser instructions (follow only if safe): ${payload.instructions}` : ""}`;
    const startedAt = Date.now();
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), PROVIDER_TIMEOUT_MS);
    let providerResponse: Response | undefined;
    try {
      try {
        providerResponse = await fetch(
          `https://generativelanguage.googleapis.com/v1beta/models/${encodeURIComponent(model)}:generateContent?key=${encodeURIComponent(apiKey)}`,
          {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            signal: controller.signal,
            body: JSON.stringify({
              system_instruction: { parts: [{ text: SEMANTIC_SYSTEM_PROMPT }] },
              contents: [{ parts: [{ text: prompt }] }],
              generationConfig: {
                temperature: 0.1,
                maxOutputTokens: Math.min(Number(payload.config.maxTokens) || 4096, 8192),
                responseMimeType: "application/json",
                responseSchema: {
                  type: "ARRAY",
                  items: {
                    type: "OBJECT",
                    properties: {
                      id: { type: "STRING" },
                      sourceMediaId: { type: "STRING" },
                      startMs: { type: "INTEGER" },
                      endMs: { type: "INTEGER" },
                      action: { type: "STRING", enum: ["CUT", "KEEP", "HIGHLIGHT"] },
                      reason: { type: "STRING" },
                      confidence: { type: "NUMBER" },
                      taxonomy: {
                        type: "STRING",
                        enum: [
                          "false_start",
                          "repeated_take",
                          "redundant_sentence",
                          "important_statement",
                          "none",
                        ],
                      },
                    },
                    required: ["startMs", "endMs", "action", "reason", "confidence", "taxonomy"],
                  },
                },
              },
            }),
          },
        );
      } catch (error) {
        const timedOut = error instanceof DOMException && error.name === "AbortError";
        console.warn("Gemini request failed", {
          requestId,
          errorCode: timedOut ? "provider_timeout" : "provider_unavailable",
        });
        return response(
          { error: timedOut ? "provider_timeout" : "provider_unavailable" },
          timedOut ? 504 : 502,
          origin,
        );
      }
    } finally {
      clearTimeout(timeout);
    }
    if (!providerResponse) return response({ error: "provider_unavailable" }, 502, origin);
    if (!providerResponse.ok) {
      console.warn("Gemini upstream failure", { requestId, status: providerResponse.status });
      return response(
        { error: providerResponse.status === 429 ? "rate_limited" : "provider_unavailable" },
        providerResponse.status === 429 ? 429 : 502,
        origin,
      );
    }
    const providerBody = await providerResponse.json();
    const text = providerBody.candidates?.[0]?.content?.parts?.[0]?.text;
    let parsed: unknown;
    try {
      parsed = JSON.parse(typeof text === "string" ? text : "[]");
    } catch {
      console.warn("Gemini returned invalid JSON", { requestId });
      return response({ error: "invalid_provider_output" }, 502, origin);
    }
    let actions: ReturnType<typeof validateActions>;
    try {
      actions = validateActions(parsed, payload);
    } catch {
      console.warn("Gemini output failed canonical validation", { requestId });
      return response({ error: "invalid_provider_output" }, 502, origin);
    }
    const finalResponse = {
      actions,
      summary: "Semantic analysis completed",
      usageTokens: providerBody.usageMetadata?.totalTokenCount ?? null,
      provider: "gemini",
      model,
      promptVersion: SEMANTIC_PROMPT_VERSION,
    };
    const { data: quotaAllowed, error: quotaError } = await admin.rpc("consume_ai_quota", {
      p_user_id: user.id,
      p_request_id: requestId,
      p_provider: "gemini",
      p_model: model,
      p_operation_type: "semantic_edit_analysis",
      p_input_chars: prompt.length,
      p_tokens_used: Number(finalResponse.usageTokens ?? 0),
      p_cost_estimate: 0,
    });
    if (quotaError) {
      console.error("quota RPC failed", { requestId });
      return response({ error: "service_unavailable" }, 503, origin);
    }
    if (!quotaAllowed) return response({ error: "quota_exceeded" }, 429, origin);
    const { error: metadataError } = await admin
      .from("ai_usage")
      .update({
        latency_ms: Math.max(0, Date.now() - startedAt),
        metadata: { response: finalResponse, promptVersion: SEMANTIC_PROMPT_VERSION },
      })
      .eq("user_id", user.id)
      .eq("request_id", requestId);
    if (metadataError) {
      console.error("usage metadata persistence failed", { requestId });
      return response({ error: "service_unavailable" }, 503, origin);
    }
    console.info("AI analysis completed", {
      requestId,
      userId: user.id,
      operation: "semantic_edit_analysis",
      latencyMs: Math.max(0, Date.now() - startedAt),
      provider: "gemini",
      model,
      usageTokens: finalResponse.usageTokens,
    });
    return response(finalResponse, 200, origin);
  } catch (error) {
    console.warn("AI analysis request rejected", {
      requestId,
      reason: error instanceof Error ? error.message : "unknown",
    });
    return response({ error: "invalid_request" }, 400, origin);
  }
});
