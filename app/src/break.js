// Chokh - চোখ | ব্রেক স্ক্রিনের লজিক
// Rust টাইমার শেষ হলে "break-start" ইভেন্ট পাঠায়; এই ফাইল কার্ড সাজিয়ে দেখায়।

const MESSAGES = [
  "২০ ফিট দূরে কোনো বস্তুর দিকে তাকান এবং ৫ বার পলক ফেলুন",
  "পিঠ সোজা করুন, কাঁধ রিল্যাক্স করুন, আরামে বসুন",
  "এক ঢোক পানি খেয়ে নিন 💧",
  "Look 20 feet away for 20 seconds",
  "সুস্থ থাকতে চাইলে এখনই ২০ সেকেন্ড রেস্ট নিন",
];
const REST_SECONDS = 20;

const $ = (id) => document.getElementById(id);
const card = $("card");
const views = { ask: $("view-ask"), count: $("view-count"), done: $("view-done") };
const bar = $("ring-bar");
const num = $("count-num");

let lastMsg = -1;      // পরপর একই মেসেজ যেন না আসে
let tick = null;       // কাউন্টডাউনের ইন্টারভাল
let finishing = false; // দুবার finish_break ডাকা ঠেকাতে

function show(name) {
  for (const [k, el] of Object.entries(views)) el.hidden = k !== name;
}

function pickMessage() {
  let i;
  do { i = Math.floor(Math.random() * MESSAGES.length); } while (i === lastMsg && MESSAGES.length > 1);
  lastMsg = i;
  return MESSAGES[i];
}

// নতুন ব্রেক শুরু: সব রিসেট
function resetCard(payload) {
  clearInterval(tick);
  finishing = false;
  document.body.classList.toggle("no-blur", !(payload && payload.blur));
  $("msg").textContent = pickMessage();
  bar.classList.remove("run");
  num.textContent = bn(REST_SECONDS);
  show("ask");
  card.classList.remove("enter");
  void card.offsetWidth; // অ্যানিমেশন আবার চালানোর কৌশল
  card.classList.add("enter");
  $("btn-rest").focus();
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
  num.textContent = bn(REST_SECONDS);
  tick = setInterval(() => {
    const left = Math.max(0, Math.ceil((endAt - Date.now()) / 1000));
    num.textContent = bn(left);
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

// Esc = Skip (ইউজার যেন কখনো আটকে না যায়)
document.addEventListener("keydown", (e) => {
  if (e.key === "Escape") finish(false);
});

listen("break-start", (event) => resetCard(event.payload));
