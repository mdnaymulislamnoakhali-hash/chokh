# Chokh - চোখ | অগ্রগতি

| # | কাজ | অবস্থা |
|---|-----|--------|
| 1 | Supabase SQL + `track` Edge Function | ✅ কোড তৈরি, GitHub-এ আপলোড |
| 2 | Tauri অ্যাপ: টাইমার, ট্রে, ব্রেক স্ক্রিন | ✅ কোড তৈরি |
| 3 | সেটিংস, ডেইলি স্ট্যাটস, সাউন্ড, ক্রেডিট, অটো-স্টার্ট, ভাষা, রিমাইন্ডার | ✅ কোড তৈরি |
| 4 | অ্যানালিটিক্স ক্লায়েন্ট (UUID, অফলাইন কিউ, নোটিস, বন্ধ করার সুইচ) | ✅ কোড তৈরি |
| 5 | অ্যাডমিন ড্যাশবোর্ড (React + Tailwind, Netlify) | ✅ কোড তৈরি (npm build এখনও চালানো হয়নি) |
| 6 | ল্যান্ডিং পেজ (দুই ভাষা, মোবাইল-ফ্রেন্ডলি) | ✅ তৈরি ও ব্রাউজারে পরীক্ষিত (ডেস্কটপ + মোবাইল) |
| 7 | GitHub Actions দিয়ে .exe বিল্ড + Releases | ⏳ পরবর্তী |
| 8 | লোগো বসানো | ⬜ (আপনি বানাচ্ছেন) |

> Rust কোড এখনো কোনো পিসিতে কম্পাইল হয়নি। প্রথম `npm run tauri dev`-এ কম্পাইল এরর এলে হুবহু এররটা পাঠান।

## 📌 আপনার জমা থাকা কাজ (একসাথে করার জন্য)

**ধাপ ১ (Supabase):**
- [ ] Supabase প্রজেক্টে `supabase/schema.sql` চালানো (আপনার অ্যাডমিন ইমেইল বসিয়ে, শুধু SQL Editor-এ; GitHub-এর ফাইলে প্লেসহোল্ডার থাকুক)
- [ ] অ্যাডমিন ইউজার বানানো ও সাইন-আপ বন্ধ করা
- [ ] `supabase functions deploy track --no-verify-jwt` ও curl টেস্ট

**ধাপ ২-৩ (অ্যাপ):**
- [ ] Node.js, Rust, Visual Studio C++ Build Tools ইনস্টল
- [ ] `app/src-tauri/src/config.rs`-এ ফেসবুক পেজের লিংক (`FACEBOOK_PAGE_URL`) ও Supabase URL (`TRACK_URL`) বসানো
- [ ] `cd app && npm install && npm run tauri dev` চালিয়ে দেখা (ব্রেক স্ক্রিন, সাউন্ড, সেটিংস)
- [ ] `package-lock.json` ও `src-tauri/Cargo.lock` কমিট করা
- [ ] (ঐচ্ছিক) বাংলা ফন্ট `src/fonts/NotoSansBengali.ttf`
- [ ] অ্যানালিটিক্স পরীক্ষা (`app/README.md`-র "অ্যানালিটিক্স পরীক্ষা" অংশ)

**ধাপ ৫ (অ্যাডমিন ড্যাশবোর্ড), `admin/README.md`-তে পুরো ধাপ আছে:**
- [ ] অ্যাডমিন ইউজার বানানোর সময় "Auto Confirm User" টিক দেওয়া
- [ ] `cd admin && cp .env.example .env`, URL ও anon key বসিয়ে `npm install && npm run dev` চালিয়ে দেখা
- [ ] `admin/package-lock.json` কমিট করা
- [ ] Netlify-তে সাইট খোলা (Base directory `admin`, Publish directory `dist`) ও দুটো `VITE_` ভ্যারিয়েবল বসানো
- [ ] ডিপ্লয় হওয়া লিংকে লগইন করে দেখা

**ধাপ ৬ (ল্যান্ডিং পেজ), `landing/README.md`-তে পুরো ধাপ আছে:**
- [ ] `landing/script.js`-এর `CONFIG`-এ ফেসবুক লিংক বসানো, আর GitHub ইউজারনেম/রিপোর নাম ঠিক আছে কি না মিলিয়ে নেওয়া
- [ ] Netlify-তে দ্বিতীয় সাইট খোলা (Base directory `landing`)
- [ ] সাইটের ঠিকানা পেয়ে `og:image` লাইন বসানো (ঐচ্ছিক)
- [ ] অ্যাপ চালানোর পর আসল স্ক্রিনশট দিয়ে `landing/assets/` ছবি বদলানো (ঐচ্ছিক)

**লোগো:**
- [ ] `assets/logo.png` (১০২৪×১০২৪, স্বচ্ছ) ও `logo.svg`, তারপর `npm run tauri icon ../assets/logo.png`
