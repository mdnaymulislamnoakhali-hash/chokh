# Supabase সেটআপ (Phase 1)

## ধাপ
1. https://supabase.com এ ফ্রি প্রজেক্ট খুলুন।
2. **SQL Editor** → `schema.sql` পুরোটা পেস্ট করে Run করুন।
   - আগে ফাইলে `YOUR_ADMIN_EMAIL@example.com` বদলে আপনার ইমেইল দিন।
3. **Authentication → Users → Add user** দিয়ে ঠিক ওই ইমেইল ও একটা শক্ত পাসওয়ার্ডে ইউজার বানান।
   (Authentication → Providers থেকে "Allow new users to sign up" বন্ধ রাখুন।)
4. Edge Function ডিপ্লয়:
   ```bash
   npm i -g supabase
   supabase login
   supabase link --project-ref <আপনার-project-ref>
   supabase functions deploy track --no-verify-jwt
   ```
5. ট্র্যাকিং URL হবে:
   `https://<project-ref>.supabase.co/functions/v1/track`
   (প্রম্পটে `/api/track` ছিল; Supabase-এ পাথ এটাই। অ্যাপে এই URL বসবে।)

## দ্রুত পরীক্ষা
```bash
curl -X POST "https://<project-ref>.supabase.co/functions/v1/track" \
  -H "Content-Type: application/json" \
  -d '{"event_id":"11111111-1111-4111-8111-111111111111","user_id":"22222222-2222-4222-8222-222222222222","event":"install","timestamp":"2026-10-28T10:00:00Z","app_version":"1.0.0","os_type":"windows"}'
# আশা করা উত্তর: {"ok":true,"accepted":1,"rejected":0}
```
একই কমান্ড আবার চালালে ডুপ্লিকেট সারি তৈরি হবে না।

## নিরাপত্তা নোট
- `analytics` টেবিলে কেউ সরাসরি পড়তে/লিখতে পারে না (RLS, কোনো policy নেই)।
- ড্যাশবোর্ড শুধু `admin_*` ফাংশন কল করবে, আর সেগুলো `admins` তালিকার ইমেইল ছাড়া `forbidden` দেয়।
- এন্ডপয়েন্ট খোলা, তাই কেউ ভুয়া ইভেন্ট পাঠাতে পারে। এখন আছে: ইভেন্ট ও UUID যাচাই, ব্যাচ সীমা (৫০), সাইজ সীমা (৩২KB)।
  বট-স্প্যাম বাড়লে Cloudflare রেট-লিমিট সামনে বসানো যাবে।
