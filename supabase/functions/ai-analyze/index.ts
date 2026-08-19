import "jsr:@supabase/functions-js/edge-runtime.d.ts"
import { createClient } from "jsr:@supabase/supabase-js@2"

const corsHeaders = {
  'Access-Control-Allow-Origin': '*',
  'Access-Control-Allow-Headers': 'authorization, x-client-info, apikey, content-type',
}

Deno.serve(async (req) => {
  // 1. CORS Preflight
  if (req.method === 'OPTIONS') {
    return new Response('ok', { headers: corsHeaders })
  }

  try {
    // 2. Auth Verification (Trust Boundary)
    const authHeader = req.headers.get('Authorization')
    if (!authHeader) {
      throw new Error('Missing Authorization header')
    }

    // Optional: You can instantiate Supabase client to verify JWT or hit DB if you need Quota check
    const supabaseClient = createClient(
      Deno.env.get('SUPABASE_URL') ?? '',
      Deno.env.get('SUPABASE_ANON_KEY') ?? '',
      { global: { headers: { Authorization: authHeader } } }
    )

    // Verify token validity by calling auth.getUser()
    const { data: { user }, error: authError } = await supabaseClient.auth.getUser()
    if (authError || !user) {
      return new Response(JSON.stringify({ error: 'Unauthorized', details: authError }), {
        status: 401,
        headers: { ...corsHeaders, 'Content-Type': 'application/json' },
      })
    }

    // 3. Payload validation
    const payload = await req.json()
    // Very basic validation (should expand for production)
    if (!payload.segments || !Array.isArray(payload.segments)) {
      throw new Error('Invalid payload: Missing or invalid segments')
    }

    // 4. Provider Call
    const apiKey = Deno.env.get('GEMINI_API_KEY')
    if (!apiKey) {
      throw new Error('Server configuration error: Missing Gemini API Key')
    }

    const modelName = 'gemini-1.5-flash'
    const url = `https://generativelanguage.googleapis.com/v1beta/models/${modelName}:generateContent?key=${apiKey}`

    // System Prompt for Semantic Analysis V1
    const systemInstruction = `You are a professional video editor analyzing a transcript. Your job is to propose conservative edits.
You MUST ONLY use the EXACT start and end timestamps provided in the transcript segments. DO NOT invent timestamps.
Apply the following taxonomy for edits:
1. 'false_start': The speaker starts a sentence, hesitates, and restarts. Action: CUT.
2. 'repeated_take': The speaker repeats the exact same phrasing due to a mistake. Action: CUT. (Intentional repetitions for emphasis should be KEEP).
3. 'redundant_sentence': Filler sentences or long dead-air that add no value. Action: CUT.
4. 'important_statement': A key hook, Call To Action, or core message. Action: HIGHLIGHT.
5. 'none': Normal dialogue. Action: KEEP.

CONSERVATIVE RULE: If you are not absolutely sure (> 80% confidence) that a segment is a mistake, you MUST default to KEEP. Do not delete content unless it is a clear error.
Provide a short, user-readable 'reason'.
`

    let transcriptData = ""
    for (const segment of payload.segments) {
      transcriptData += `[${segment.start.toFixed(2)} - ${segment.end.toFixed(2)}] ${segment.text}\n`
    }

    if (payload.instructions) {
      transcriptData += `\nUser Custom Instructions: ${payload.instructions}\n`
    }

    const geminiReq = {
      system_instruction: {
        parts: [{ text: systemInstruction }]
      },
      contents: [{
        parts: [{ text: transcriptData }],
      }],
      generationConfig: {
        responseMimeType: "application/json",
        responseSchema: {
          type: "ARRAY",
          items: {
            type: "OBJECT",
            properties: {
              start: { type: "NUMBER", description: "Exact start timestamp from input" },
              end: { type: "NUMBER", description: "Exact end timestamp from input" },
              action: { type: "STRING", enum: ["CUT", "KEEP", "HIGHLIGHT"], description: "Proposed edit action" },
              reason: { type: "STRING", description: "Short human-readable explanation" },
              confidence: { type: "NUMBER", description: "0.0 to 1.0 representing certainty" },
              taxonomy: { type: "STRING", enum: ["false_start", "repeated_take", "redundant_sentence", "important_statement", "none"] }
            },
            required: ["start", "end", "action", "reason", "confidence", "taxonomy"]
          }
        }
      }
    }

    const response = await fetch(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(geminiReq)
    })

    if (!response.ok) {
      const errBody = await response.text()
      console.error(`Gemini Error ${response.status}:`, errBody)
      return new Response(JSON.stringify({ error: 'Provider API Error', code: response.status }), {
        status: 502, // Bad Gateway (Upstream error)
        headers: { ...corsHeaders, 'Content-Type': 'application/json' },
      })
    }

    const body = await response.json()
    let text = body.candidates?.[0]?.content?.parts?.[0]?.text || '[]'

    // JSON parsing without stripping since responseMimeType guarantees JSON output
    let actions = []
    try {
      actions = JSON.parse(text)
    } catch (e) {
      console.error('Failed to parse Gemini JSON output', e)
      return new Response(JSON.stringify({ error: 'Invalid provider output format' }), {
        status: 422, // Unprocessable Entity
        headers: { ...corsHeaders, 'Content-Type': 'application/json' },
      })
    }

    // Success response matching AIAnalysisResponse
    const finalResponse = {
      actions: actions,
      summary: "Gemini analysis completed via Edge Function",
      usage_tokens: null
    }

    return new Response(JSON.stringify(finalResponse), {
      status: 200,
      headers: { ...corsHeaders, 'Content-Type': 'application/json' },
    })

  } catch (err: any) {
    console.error(err)
    return new Response(JSON.stringify({ error: err.message }), {
      status: 400,
      headers: { ...corsHeaders, 'Content-Type': 'application/json' },
    })
  }
})
