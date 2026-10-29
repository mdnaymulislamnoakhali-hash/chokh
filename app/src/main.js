// Chokh - চোখ | মূল (স্ট্যাটাস) জানালা
// 🔜 ধাপ ৩-এ এই জানালা পূর্ণ সেটিংস পাতা (সাউন্ড, অটো-স্টার্ট, ভাষা, রিমাইন্ডার) ও দৈনিক স্ট্যাটসে বদলাবে।

const $ = (id) => document.getElementById(id);

const els = {
  label: $("timer-label"),
  time: $("timer-time"),
  paused: $("paused-note"),
  seg: $("interval-seg"),
  pause1: $("pause-1"),
  pause2: $("pause-2"),
  resume: $("resume"),
  test: $("test-break"),
};

function mmss(secs) {
  const m = Math.floor(secs / 60);
  const s = secs % 60;
  return bn(String(m).padStart(2, "0") + ":" + String(s).padStart(2, "0"));
}

function humanLeft(secs) {
  const h = Math.floor(secs / 3600);
  const m = Math.ceil((secs % 3600) / 60);
  if (h > 0) return bn(h) + " ঘণ্টা " + bn(m) + " মিনিট";
  return bn(Math.max(m, 1)) + " মিনিট";
}

async function refresh() {
  if (document.hidden) return; // জানালা লুকানো থাকলে কাজ বাঁচাই
  let st;
  try {
    st = await invoke("get_status");
  } catch (e) {
    els.time.textContent = "--:--";
    return;
  }

  const isPaused = st.pause_secs_left > 0;
  els.time.textContent = isPaused ? "⏸" : mmss(st.secs_until_break);
  els.label.textContent = isPaused ? "রিমাইন্ডার বিরতিতে" : "পরের ব্রেক";
  els.paused.hidden = !isPaused;
  if (isPaused) els.paused.textContent = "আর " + humanLeft(st.pause_secs_left) + " পর নিজে চালু হবে";

  els.resume.hidden = !isPaused;
  els.pause1.disabled = isPaused;
  els.pause2.disabled = isPaused;

  for (const b of els.seg.querySelectorAll("button")) {
    b.setAttribute("aria-pressed", String(Number(b.dataset.min) === st.interval_min));
  }
}

els.seg.addEventListener("click", async (ev) => {
  const btn = ev.target.closest("button[data-min]");
  if (!btn) return;
  await invoke("set_interval", { minutes: Number(btn.dataset.min) });
  refresh();
});
els.pause1.addEventListener("click", async () => { await invoke("pause", { hours: 1 }); refresh(); });
els.pause2.addEventListener("click", async () => { await invoke("pause", { hours: 2 }); refresh(); });
els.resume.addEventListener("click", async () => { await invoke("resume"); refresh(); });
els.test.addEventListener("click", () => invoke("start_break_now"));

document.addEventListener("visibilitychange", refresh);
setInterval(refresh, 1000);
refresh();
