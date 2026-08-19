-- 1. Chống ghi đè trùng lặp (Idempotency) trên Credit Ledger
ALTER TABLE credit_ledger
ADD CONSTRAINT unique_transaction_reference UNIQUE (reference_id, transaction_type);

-- 2. Trigger cấp tự động Trial Credit (50,000 Tokens) khi User tạo tài khoản mới
CREATE OR REPLACE FUNCTION on_auth_user_created()
RETURNS TRIGGER 
LANGUAGE plpgsql
SECURITY DEFINER
AS $$
BEGIN
    -- Tạo một hồ sơ profile cơ bản
    INSERT INTO public.profiles (id, email)
    VALUES (new.id, new.email);
    
    -- Tặng 50,000 Trial Credits (Tương đương 50k tokens)
    INSERT INTO public.credit_ledger (user_id, amount, reason, transaction_type)
    VALUES (new.id, 50000, 'Free Trial Credits for New Signup', 'trial/bonus');
    
    RETURN new;
END;
$$;

-- Gắn trigger vào bảng auth.users
DROP TRIGGER IF EXISTS on_auth_user_created_trigger ON auth.users;
CREATE TRIGGER on_auth_user_created_trigger
AFTER INSERT ON auth.users
FOR EACH ROW EXECUTE FUNCTION on_auth_user_created();

-- 3. Cập nhật lại hàm Commit Credits có chèn thêm ai_usage (Log Token/Model)
-- Xóa hàm cũ đi trước khi tái tạo với chữ ký mới
DROP FUNCTION IF EXISTS commit_credits(UUID, INT, UUID);

CREATE OR REPLACE FUNCTION commit_credits(
    p_user_id UUID, 
    p_amount_used INT, 
    p_request_id UUID, 
    p_provider TEXT, 
    p_model TEXT, 
    p_cost_usd NUMERIC(10, 6) DEFAULT 0
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
AS $$
DECLARE
    reserved_amount INT;
BEGIN
    -- Tìm xem lượng đã reserve là bao nhiêu (số âm)
    SELECT amount INTO reserved_amount
    FROM credit_ledger
    WHERE user_id = p_user_id 
      AND reference_id = p_request_id 
      AND transaction_type = 'reservation'
    LIMIT 1;

    -- Nếu tìm thấy và lượng reserve > lượng xài thực tế, hoàn lại phần thừa
    IF reserved_amount IS NOT NULL THEN
        IF ABS(reserved_amount) > p_amount_used THEN
            INSERT INTO credit_ledger (user_id, amount, reason, transaction_type, reference_id)
            VALUES (p_user_id, ABS(reserved_amount) - p_amount_used, 'Partial Refund (Over-reserved)', 'refund', p_request_id)
            ON CONFLICT DO NOTHING; -- Tránh chèn đúp nếu lỡ gọi 2 lần
        END IF;

        -- Đánh dấu dòng cũ thành usage_commit
        UPDATE credit_ledger
        SET transaction_type = 'usage_commit'
        WHERE user_id = p_user_id 
          AND reference_id = p_request_id 
          AND transaction_type = 'reservation';
          
        -- Chèn log chi tiết vào ai_usage
        INSERT INTO ai_usage (user_id, provider, model, tokens_used, cost_usd)
        VALUES (p_user_id, p_provider, p_model, p_amount_used, p_cost_usd);
    END IF;
END;
$$;
