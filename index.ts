// =====================================================================
// Chokh - চোখ | POST /functions/v1/track
// ডেস্কটপ অ্যাপ থেকে বেনামী ইভেন্ট নিয়ে analytics টেবিলে রাখে।
// একটা ইভেন্ট অথবা অফলাইন কিউ থেকে ব্যাচ (সর্বোচ্চ ৫০টি) গ্রহণ করে।
// =====================================================================
import { createClient } from "https://esm.sh/@supabase/supabase-js@2.45.4";

const ALLOWED_EVENTS = new Set(["install", "app_open", "break_taken", "break_skipped"]);
const UUID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
const MAX_BATCH = 50;
const MAX_BODY_BYTES = 32 * 1024;
const MAX_AGE_MS = 60 * 24 * 60 * 60 * 1000; // ৬০ দিনের পুরনো ইভেন্ট সার্ভারের সময় ধরা হবে
const MAX_FUTURE_MS = 5 * 60 * 1000;         // ভবিষ্যতের টাইমস্ট্যাম্প সার্ভারের সময় ধরা হবে

const supabase = createClient(
  Deno.env.get("SUPABASE_URL")!,
  Deno.env.get("SUPABASE_SERVICE_ROLE_KEY")!,
  { auth: { persistSession: false } },
);

// Tauri WebView-এর origin আলাদা, তাই CORS খোলা। কোনো সিক্রেট ডেটা রিটার্ন হয় না।
const CORS = {
  "Access-Control-Allow-Origin": "*",
  "Access-Control-Allow-Methods": "POST, OPTIONS",
  "Access-Control-Allow-Headers": "content-type",
};

function json(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { ...CORS, "Content-Type": "application/json" },
  });
}

function cleanText(v: unknown, fallback: string): string {
  if (typeof v !== "string") return fallback;
  const s = v.trim().slice(0, 20);
  return /^[\w.\-+ ]+$/.test(s) ? s : fallback;
}

// একটা ইভেন্ট যাচাই করে DB-র সারি বানায়; অবৈধ হলে null
function normalize(raw: any, now: number) {
  if (!raw || typeof raw !== "object") return null;
  if (typeof raw.user_id !== "string" || !UUID_RE.test(raw.user_id)) return null;
  if (typeof raw.event !== "string" || !ALLOWED_EVENTS.has(raw.event)) return null;

  const eventId =
    typeof raw.event_id === "string" && UUID_RE.test(raw.event_id)
      ? raw.event_id
      : crypto.randomUUID();

  let ts = Date.parse(String(raw.timestamp ?? ""));
  if (Number.isNaN(ts) || ts > now + MAX_FUTURE_MS || ts < now - MAX_AGE_MS) ts = now;

  return {
    event_id: eventId,
    user_id: raw.user_id,
    event: raw.event,
    event_ts: new Date(ts).toISOString(),
    app_version: cleanText(raw.app_version, "unknown"),
    os_type: cleanText(raw.os_type, "unknown"),
  };
}

Deno.serve(async (req) => {
  if (req.method === "OPTIONS") return new Response(null, { status: 204, headers: CORS });
  if (req.method !== "POST") return json({ ok: false, error: "method_not_allowed" }, 405);

  const text = await req.text();
  if (text.length > MAX_BODY_BYTES) return json({ ok: false, error: "too_large" }, 413);

  let payload: any;
  try {
    payload = JSON.parse(text);
  } catch {
    return json({ ok: false, error: "bad_json" }, 400);
  }

  // তিনটি আকার গ্রহণযোগ্য: {…}, […], {"events":[…]}
  const list: unknown[] = Array.isArray(payload)
    ? payload
    : Array.isArray(payload?.events)
    ? payload.events
    : [payload];

  if (list.length === 0 || list.length > MAX_BATCH) {
    return json({ ok: false, error: "bad_batch_size" }, 400);
  }

  const now = Date.now();
  const rows = list.map((e) => normalize(e, now)).filter((r) => r !== null);
  if (rows.length === 0) return json({ ok: false, error: "no_valid_events" }, 400);

  // event_id ইউনিক: একই ইভেন্ট দ্বিতীয়বার এলে চুপচাপ উপেক্ষা করা হয়
  const { error } = await supabase
    .from("analytics")
    .upsert(rows, { onConflict: "event_id", ignoreDuplicates: true });

  if (error) {
    console.error("insert failed:", error.message);
    return json({ ok: false, error: "server_error" }, 500);
  }
  return json({ ok: true, accepted: rows.length, rejected: list.length - rows.length });
});
