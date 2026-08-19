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

    // Reconstruct prompt exactly like Desktop did
    let prompt = "Analyze the following transcript and suggest edits (CUT/KEEP/HIGHLIGHT).\n"
    prompt += "Return a JSON array of objects with fields: start (float), end (float), action (string), reason (string).\n\n"
    
    for (const segment of payload.segments) {
      prompt += `[${segment.start.toFixed(2)} - ${segment.end.toFixed(2)}] ${segment.text}\n`
    }

    if (payload.instructions) {
      prompt += `\nUser Instructions: ${payload.instructions}\n`
    }

    const geminiReq = {
      contents: [{
        parts: [{ text: prompt }],
      }],
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
    let text = body.candidates?.[0]?.content?.parts?.[0]?.text || ''

    // 5. Output Validation Hook
    // Strip markdown backticks
    text = text.trim()
    if (text.startsWith('```json')) text = text.substring(7)
    if (text.startsWith('```')) text = text.substring(3)
    if (text.endsWith('```')) text = text.substring(0, text.length - 3)
    text = text.trim()

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
