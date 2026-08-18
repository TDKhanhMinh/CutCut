-- Migration: Initial Schema for CutCut Backend
-- Description: Auth profiles, entitlements, devices, credit_ledger, ai_usage, app_config

-- 1. Create tables

CREATE TABLE public.profiles (
    id uuid REFERENCES auth.users(id) ON DELETE CASCADE PRIMARY KEY,
    display_name text,
    avatar_url text,
    created_at timestamptz DEFAULT now() NOT NULL,
    updated_at timestamptz DEFAULT now() NOT NULL
);

CREATE TABLE public.entitlements (
    id uuid DEFAULT gen_random_uuid() PRIMARY KEY,
    user_id uuid REFERENCES auth.users(id) ON DELETE CASCADE NOT NULL,
    plan_id text NOT NULL,
    features jsonb DEFAULT '{}'::jsonb NOT NULL,
    expires_at timestamptz,
    created_at timestamptz DEFAULT now() NOT NULL
);

CREATE TABLE public.devices (
    id uuid DEFAULT gen_random_uuid() PRIMARY KEY,
    user_id uuid REFERENCES auth.users(id) ON DELETE CASCADE NOT NULL,
    device_hash text NOT NULL,
    last_active_at timestamptz DEFAULT now() NOT NULL,
    is_revoked boolean DEFAULT false NOT NULL,
    UNIQUE(user_id, device_hash)
);

CREATE TABLE public.credit_ledger (
    id uuid DEFAULT gen_random_uuid() PRIMARY KEY,
    user_id uuid REFERENCES auth.users(id) ON DELETE CASCADE NOT NULL,
    amount integer NOT NULL, -- positive for grant, negative for usage
    transaction_type text NOT NULL, -- e.g. 'GRANT', 'USAGE', 'REFUND'
    reference_id text,
    created_at timestamptz DEFAULT now() NOT NULL
);

CREATE TABLE public.ai_usage (
    id uuid DEFAULT gen_random_uuid() PRIMARY KEY,
    user_id uuid REFERENCES auth.users(id) ON DELETE CASCADE NOT NULL,
    provider text NOT NULL,
    model text NOT NULL,
    tokens_used integer DEFAULT 0 NOT NULL,
    cost_estimate numeric(10, 6) DEFAULT 0 NOT NULL,
    operation_type text NOT NULL,
    created_at timestamptz DEFAULT now() NOT NULL
);

CREATE TABLE public.app_config (
    key text PRIMARY KEY,
    value jsonb NOT NULL,
    updated_at timestamptz DEFAULT now() NOT NULL
);

-- 2. Indexes

CREATE INDEX idx_entitlements_user_id ON public.entitlements(user_id);
CREATE INDEX idx_devices_user_id ON public.devices(user_id);
CREATE INDEX idx_credit_ledger_user_id ON public.credit_ledger(user_id);
CREATE INDEX idx_ai_usage_user_id ON public.ai_usage(user_id);

-- 3. Row Level Security (RLS) Policies

ALTER TABLE public.profiles ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.entitlements ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.devices ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.credit_ledger ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.ai_usage ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.app_config ENABLE ROW LEVEL SECURITY;

-- Profiles: Users can read and update their own profile
CREATE POLICY "Users can view own profile" 
    ON public.profiles FOR SELECT 
    USING (auth.uid() = id);

CREATE POLICY "Users can update own profile" 
    ON public.profiles FOR UPDATE 
    USING (auth.uid() = id);

-- Entitlements: Users can only read their own entitlements
CREATE POLICY "Users can view own entitlements" 
    ON public.entitlements FOR SELECT 
    USING (auth.uid() = user_id);

-- Devices: Users can read and update (e.g. revoke) their own devices
CREATE POLICY "Users can view own devices" 
    ON public.devices FOR SELECT 
    USING (auth.uid() = user_id);

CREATE POLICY "Users can insert own device" 
    ON public.devices FOR INSERT 
    WITH CHECK (auth.uid() = user_id);

CREATE POLICY "Users can update own devices" 
    ON public.devices FOR UPDATE 
    USING (auth.uid() = user_id);

-- Credit Ledger: Users can only read their ledger (backend handles inserts)
CREATE POLICY "Users can view own credit ledger" 
    ON public.credit_ledger FOR SELECT 
    USING (auth.uid() = user_id);

-- AI Usage: Users can read their usage logs
CREATE POLICY "Users can view own ai usage" 
    ON public.ai_usage FOR SELECT 
    USING (auth.uid() = user_id);

-- App Config: Anyone can read config
CREATE POLICY "Anyone can view app config" 
    ON public.app_config FOR SELECT 
    TO public
    USING (true);

-- Note: INSERT/UPDATE/DELETE on entitlements, credit_ledger, ai_usage, app_config 
-- are strictly allowed ONLY for the service_role (backend server/edge functions), 
-- which bypasses RLS by default.

-- 4. Triggers

-- Trigger function to automatically create a profile for a new user
CREATE OR REPLACE FUNCTION public.handle_new_user()
RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO public.profiles (id, display_name, avatar_url)
    VALUES (
        new.id,
        COALESCE(new.raw_user_meta_data->>'full_name', new.email),
        new.raw_user_meta_data->>'avatar_url'
    );
    RETURN new;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

CREATE TRIGGER on_auth_user_created
    AFTER INSERT ON auth.users
    FOR EACH ROW EXECUTE FUNCTION public.handle_new_user();
