// Chokh - চোখ | দুই জানালার শেয়ার্ড হেলপার

// Tauri গ্লোবাল API (tauri.conf.json-এ withGlobalTauri: true থাকায় পাওয়া যায়)
const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

// ইংরেজি অঙ্ক → বাংলা অঙ্ক
const BN_DIGITS = "০১২৩৪৫৬৭৮৯";
function bn(value) {
  return String(value).replace(/\d/g, (d) => BN_DIGITS[d]);
}
