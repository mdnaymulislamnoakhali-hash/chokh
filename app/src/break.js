// Chokh - চোখ | ব্রেক স্ক্রিনের লজিক
// Rust টাইমার শেষ হলে "break-start" ইভেন্ট পাঠায়; এই ফাইল কার্ড সাজিয়ে দেখায়।

const REST_SECONDS = 20;

const $ = (id) => document.getElementById(id);
const card = $("card");
const views = { ask: $("view-ask"), count: $("view-count"), done: $("view-done") };
const bar = $("ring-bar");
const countNum = $("count-num");

let lastMsg = -1;      // পরপর একই মেসেজ যেন না আসে
let tick = null;       // কাউন্টডাউনের ইন্টারভাল
let finishing = false; // দুবার finish_break ডাকা ঠেকাতে

function show(name) {
  for (const [k, el] of Object.entries(views)) el.hidden = k !== name;
}

function pickMessage() {
  const list = t("msgs");
  let i;
  do { i = Math.floor(Math.random() * list.length); } while (i === lastMsg && list.length > 1);
  lastMsg = i;
  return list[i];
}

// নরম "টিং": দুটো সাইন তরঙ্গ, দ্রুত উঠে ধীরে মিলিয়ে যায়। কোনো সাউন্ড ফাইল লাগে না।
function playTing() {
  try {
    const Ctx = window.AudioContext || window.webkitAudioContext;
    const ctx = new Ctx();
    const t0 = ctx.currentTime;
    const master = ctx.createGain();
    master.gain.setValueAtTime(0.0001, t0);
    master.gain.exponentialRampToValueAtTime(0.16, t0 + 0.012);
    master.gain.exponentialRampToValueAtTime(0.0001, t0 + 1.7);
    master.connect(ctx.destination);
    for (const [freq, level] of [[880, 1], [1760, 0.32], [2640, 0.1]]) {
      const osc = ctx.createOscillator();
      const g = ctx.createGain();
      osc.type = "sine";
      osc.frequency.value = freq;
      g.gain.value = level;
      osc.connect(g);
      g.connect(master);
      osc.start(t0);
      osc.stop(t0 + 1.8);
    }
    setTimeout(() => ctx.close(), 2200);
  } catch (e) {
    /* সাউন্ড না বাজলে অ্যাপ থামবে না */
  }
}

// নতুন ব্রেক শুরু: সব রিসেট
function resetCard(p) {
  clearInterval(tick);
  finishing = false;
  setLang(p.lang);
  applyI18n();

  document.body.classList.toggle("no-blur", !p.blur);
  $("msg").textContent = pickMessage();
  $("stats").textContent = t("today", { a: num(p.taken), b: num(p.skipped) });

  // ক্রেডিট (Rust ঠিক করে দেয় আজ দেখানো হবে কি না)
  $("credit").hidden = !p.credit;
  $("credit-text").textContent = t("credit");
  const follow = $("btn-follow");
  follow.textContent = t("follow");
  follow.hidden = !(p.credit && p.fb_ready);

  bar.classList.remove("run");
  countNum.textContent = num(REST_SECONDS);
  show("ask");
  card.classList.remove("enter");
  void card.offsetWidth; // অ্যানিমেশন আবার চালানোর কৌশল
  card.classList.add("enter");
  $("btn-rest").focus();

  if (p.sound) playTing();
}

async function finish(taken) {
  if (finishing) return;
  finishing = true;
  clearInterval(tick);
  await invoke("finish_break", { taken });
}

// ২০ সেকেন্ডের কাউন্টডাউন
function startRest() {
  show("count");
  bar.classList.remove("run");
  void bar.getBoundingClientRect();
  bar.classList.add("run");

  const endAt = Date.now() + REST_SECONDS * 1000;
  countNum.textContent = num(REST_SECONDS);
  tick = setInterval(() => {
    const left = Math.max(0, Math.ceil((endAt - Date.now()) / 1000));
    countNum.textContent = num(left);
    if (left === 0) {
      clearInterval(tick);
      show("done");
      setTimeout(() => finish(true), 1600);
    }
  }, 250);
}

$("btn-rest").addEventListener("click", startRest);
$("btn-skip").addEventListener("click", () => finish(false));
$("btn-stop").addEventListener("click", () => finish(false));
$("btn-follow").addEventListener("click", () => {
  if (finishing) return;
  finishing = true;
  clearInterval(tick);
  invoke("open_facebook"); // Rust লিংক খুলে ব্রেক বন্ধ করে দেয়, যাতে ব্রাউজার দেখা যায়
});

// Esc = Skip (ইউজার যেন কখনো আটকে না যায়)
document.addEventListener("keydown", (e) => {
  if (e.key === "Escape") finish(false);
});

listen("break-start", (event) => resetCard(event.payload));
