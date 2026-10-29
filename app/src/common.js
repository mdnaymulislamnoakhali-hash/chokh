// Chokh - চোখ | দুই জানালার শেয়ার্ড হেলপার: Tauri API, বাংলা অঙ্ক, ভাষা (i18n)

// Tauri গ্লোবাল API (tauri.conf.json-এ withGlobalTauri: true থাকায় পাওয়া যায়)
const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

// ইংরেজি অঙ্ক → বাংলা অঙ্ক
const BN_DIGITS = "০১২৩৪৫৬৭৮৯";
function bn(value) {
  return String(value).replace(/\d/g, (d) => BN_DIGITS[d]);
}

// ---------------------------------------------------------------------
// ভাষা: সব লেখা এখানে। {a}, {b} জায়গায় মান বসে।
// ---------------------------------------------------------------------
const I18N = {
  bn: {
    tagline: "২০ মিনিট কাজ, ২০ ফিট দূরে, ২০ সেকেন্ড বিশ্রাম",
    next_break: "পরের ব্রেক",
    paused_title: "রিমাইন্ডার বিরতিতে",
    paused_note: "আর {a} পর নিজে চালু হবে",
    today: "আজ: {a} বার রেস্ট, {b} বার Skip",
    interval_h: "কতক্ষণ পরপর রিমাইন্ড করব?",
    minutes: "মিনিট",
    prefs_h: "পছন্দ",
    sound: "ব্রেকের সময় ‘টিং’ সাউন্ড",
    autostart: "Windows চালু হলে নিজে চালু হোক",
    water: "পানি খাওয়ার রিমাইন্ডার (প্রতি ৬০ মিনিটে)",
    posture: "বসার ভঙ্গির রিমাইন্ডার (প্রতি ৪৫ মিনিটে)",
    lang_h: "ভাষা",
    pause_h: "এখন বিরক্ত করবেন না",
    pause1: "১ ঘণ্টা বিরতি",
    pause2: "২ ঘণ্টা বিরতি",
    resume: "রিমাইন্ডার চালু করুন",
    test: "ব্রেক স্ক্রিন কেমন দেখায়, দেখে নিন",
    footer1: "✕ চাপলে অ্যাপ বন্ধ হয় না, ট্রে-তে চুপচাপ চলতে থাকে।",
    footer2: "পুরোপুরি বন্ধ করতে ট্রে আইকনে ডান ক্লিক → বন্ধ করুন।",
    hours: "ঘণ্টা",
    mins: "মিনিট",
    // ব্রেক স্ক্রিন
    title: "চোখকে ভালোবাসুন",
    rest: "✅ রেস্ট নিলাম (20s)",
    skip: "⏭️ Skip",
    count_msg: "২০ ফিট দূরের কিছুর দিকে তাকিয়ে থাকুন, আর মাঝে মাঝে পলক ফেলুন।",
    stop: "আগেই শেষ করুন",
    done: "চোখ বিশ্রাম পেল। এবার কাজে ফিরুন 🌿",
    credit: "এই সফটওয়্যারটি তৈরি করেছে নাঈম নান্দনিক ❤️\nভালো লাগলে পেজটি ফলো করে উৎসাহ দিন",
    follow: "Facebook Page Follow করুন",
    msgs: [
      "২০ ফিট দূরে কোনো বস্তুর দিকে তাকান এবং ৫ বার পলক ফেলুন",
      "পিঠ সোজা করুন, কাঁধ রিল্যাক্স করুন, আরামে বসুন",
      "এক ঢোক পানি খেয়ে নিন 💧",
      "Look 20 feet away for 20 seconds",
      "সুস্থ থাকতে চাইলে এখনই ২০ সেকেন্ড রেস্ট নিন",
    ],
  },
  en: {
    tagline: "20 minutes of work, 20 feet away, 20 seconds of rest",
    next_break: "Next break in",
    paused_title: "Reminders paused",
    paused_note: "Resumes automatically in {a}",
    today: "Today: {a} breaks taken, {b} skipped",
    interval_h: "Remind me every",
    minutes: "minutes",
    prefs_h: "Preferences",
    sound: "Soft ‘ting’ sound on break",
    autostart: "Start with Windows",
    water: "Water reminder (every 60 min)",
    posture: "Posture reminder (every 45 min)",
    lang_h: "Language",
    pause_h: "Do not disturb",
    pause1: "Pause 1 hour",
    pause2: "Pause 2 hours",
    resume: "Resume reminders",
    test: "Preview the break screen",
    footer1: "Closing the window keeps Chokh running in the tray.",
    footer2: "To quit completely, right-click the tray icon → Quit.",
    hours: "h",
    mins: "min",
    title: "Be kind to your eyes",
    rest: "✅ I took a rest (20s)",
    skip: "⏭️ Skip",
    count_msg: "Look at something 20 feet away and blink now and then.",
    stop: "Finish early",
    done: "Your eyes had a rest. Back to work 🌿",
    credit: "This software was made by Naeem Nandanik ❤️\nIf you like it, follow the page to encourage me",
    follow: "Follow on Facebook",
    msgs: [
      "Look at something 20 feet away and blink 5 times",
      "Straighten your back, relax your shoulders, sit comfortably",
      "Take a sip of water 💧",
      "Look 20 feet away for 20 seconds",
      "Want to stay healthy? Take a 20-second rest now",
    ],
  },
};

let LANG = "bn";
function setLang(l) {
  LANG = I18N[l] ? l : "bn";
  document.documentElement.lang = LANG;
}
// লেখা খোঁজা; না পেলে বাংলা, তাও না পেলে কী-টাই
function t(key, vars) {
  let s = (I18N[LANG] && I18N[LANG][key]) ?? I18N.bn[key] ?? key;
  if (vars && typeof s === "string") {
    for (const [k, v] of Object.entries(vars)) s = s.replace("{" + k + "}", v);
  }
  return s;
}
// ভাষা অনুযায়ী সংখ্যা: বাংলায় বাংলা অঙ্ক
function num(v) {
  return LANG === "bn" ? bn(v) : String(v);
}
// data-i18n="কী" দেওয়া সব উপাদানের লেখা বদলে দেয়
function applyI18n() {
  for (const el of document.querySelectorAll("[data-i18n]")) {
    el.textContent = t(el.dataset.i18n);
  }
}
