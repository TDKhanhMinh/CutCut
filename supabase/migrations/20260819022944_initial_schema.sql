-- 1. Profiles
CREATE TABLE profiles (
    id UUID PRIMARY KEY REFERENCES auth.users(id) ON DELETE CASCADE,
    email TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);
ALTER TABLE profiles ENABLE ROW LEVEL SECURITY;
CREATE POLICY "Users can read own profile" ON profiles FOR SELECT USING (auth.uid() = id);

-- 2. Entitlements
CREATE TABLE entitlements (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
    plan TEXT NOT NULL DEFAULT 'FREE',
    capabilities TEXT[] NOT NULL DEFAULT '{}',
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW()
);
ALTER TABLE entitlements ENABLE ROW LEVEL SECURITY;
CREATE POLICY "Users can read own entitlements" ON entitlements FOR SELECT USING (auth.uid() = user_id);

-- 3. Devices
CREATE TABLE devices (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
    installation_id UUID NOT NULL UNIQUE,
    platform TEXT,
    last_seen TIMESTAMPTZ DEFAULT NOW(),
    created_at TIMESTAMPTZ DEFAULT NOW()
);
ALTER TABLE devices ENABLE ROW LEVEL SECURITY;
CREATE POLICY "Users can manage own devices" ON devices FOR ALL USING (auth.uid() = user_id);

-- 4. Credit Ledger (Append-only)
CREATE TABLE credit_ledger (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
    amount INT NOT NULL, -- Dương (Nạp), Âm (Tiêu)
    reason TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);
ALTER TABLE credit_ledger ENABLE ROW LEVEL SECURITY;
CREATE POLICY "Users can read own ledger" ON credit_ledger FOR SELECT USING (auth.uid() = user_id);
-- (Insert chỉ dành cho Server/Edge Functions)

-- 5. AI Usage
CREATE TABLE ai_usage (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
    provider TEXT NOT NULL,
    model TEXT NOT NULL,
    tokens_used INT NOT NULL,
    cost_usd NUMERIC(10,6),
    created_at TIMESTAMPTZ DEFAULT NOW()
);
ALTER TABLE ai_usage ENABLE ROW LEVEL SECURITY;
CREATE POLICY "Users can read own usage" ON ai_usage FOR SELECT USING (auth.uid() = user_id);
