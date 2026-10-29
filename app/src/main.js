// Chokh - চোখ | মূল জানালা: স্ট্যাটাস + সেটিংস

const $ = (id) => document.getElementById(id);

const els = {
  label: $("timer-label"),
  time: $("timer-time"),
  paused: $("paused-note"),
  today: $("today-line"),
  seg: $("interval-seg"),
  langSeg: $("lang-seg"),
  pause1: $("pause-1"),
  pause2: $("pause-2"),
  resume: $("resume"),
  test: $("test-break"),
};

let current = null;   // সর্বশেষ জানা সেটিংস
let shownLang = null; // কোন ভাষায় লেখা বসানো আছে

function mmss(secs) {
  const m = Math.floor(secs / 60);
  const s = secs % 60;
  return num(String(m).padStart(2, "0") + ":" + String(s).padStart(2, "0"));
}

function humanLeft(secs) {
  const h = Math.floor(secs / 3600);
  const m = Math.ceil((secs % 3600) / 60);
  if (h > 0) return num(h) + " " + t("hours") + " " + num(m) + " " + t("mins");
  return num(Math.max(m, 1)) + " " + t("mins");
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
  current = st.settings;

  if (shownLang !== current.lang) {
    setLang(current.lang);
    applyI18n();
    shownLang = current.lang;
  }

  const isPaused = st.pause_secs_left > 0;
  els.time.textContent = isPaused ? "⏸" : mmss(st.secs_until_break);
  els.label.textContent = isPaused ? t("paused_title") : t("next_break");
  els.paused.hidden = !isPaused;
  if (isPaused) els.paused.textContent = t("paused_note", { a: humanLeft(st.pause_secs_left) });
  els.today.textContent = t("today", { a: num(st.today_taken), b: num(st.today_skipped) });

  els.resume.hidden = !isPaused;
  els.pause1.disabled = isPaused;
  els.pause2.disabled = isPaused;

  for (const b of els.seg.querySelectorAll("button")) {
    const m = Number(b.dataset.min);
    b.textContent = num(m);
    b.setAttribute("aria-pressed", String(m === current.interval_min));
  }
  for (const b of els.langSeg.querySelectorAll("button")) {
    b.setAttribute("aria-pressed", String(b.dataset.lang === current.lang));
  }
  for (const tg of document.querySelectorAll(".toggle")) {
    tg.setAttribute("aria-checked", String(Boolean(current[tg.dataset.key])));
  }
}

// একটা বা কয়েকটা সেটিং বদলে সঙ্গে সঙ্গে সেভ
async function save(patch) {
  if (!current) return;
  await invoke("set_settings", { settings: { ...current, ...patch } });
  refresh();
}

els.seg.addEventListener("click", (ev) => {
  const btn = ev.target.closest("button[data-min]");
  if (btn) save({ interval_min: Number(btn.dataset.min) });
});
els.langSeg.addEventListener("click", (ev) => {
  const btn = ev.target.closest("button[data-lang]");
  if (btn) save({ lang: btn.dataset.lang });
});
document.addEventListener("click", (ev) => {
  const tg = ev.target.closest(".toggle");
  if (tg && current) save({ [tg.dataset.key]: !current[tg.dataset.key] });
});
els.pause1.addEventListener("click", async () => { await invoke("pause", { hours: 1 }); refresh(); });
els.pause2.addEventListener("click", async () => { await invoke("pause", { hours: 2 }); refresh(); });
els.resume.addEventListener("click", async () => { await invoke("resume"); refresh(); });
els.test.addEventListener("click", () => invoke("start_break_now"));

document.addEventListener("visibilitychange", refresh);
setInterval(refresh, 1000);
refresh();
