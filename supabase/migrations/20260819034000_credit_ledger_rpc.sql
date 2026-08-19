-- 1. Bổ sung column vào credit_ledger (nếu chưa có)
-- Vì file initial_schema có thể thiếu transaction_type và reference_id, ta bổ sung hoặc thay đổi
ALTER TABLE credit_ledger
ADD COLUMN IF NOT EXISTS transaction_type TEXT CHECK (transaction_type IN ('purchase', 'reservation', 'usage_commit', 'refund', 'adjustment')) DEFAULT 'adjustment',
ADD COLUMN IF NOT EXISTS reference_id UUID;

-- 2. Tạo View để tính toán số dư hiện tại của User
CREATE OR REPLACE VIEW user_balances AS
SELECT user_id, COALESCE(SUM(amount), 0) AS balance
FROM credit_ledger
GROUP BY user_id;

-- 3. Hàm Reserve Credits (Tạm giữ tiền)
-- Trả về TRUE nếu đủ tiền, FALSE nếu không đủ
CREATE OR REPLACE FUNCTION reserve_credits(p_user_id UUID, p_amount INT, p_request_id UUID)
RETURNS BOOLEAN
LANGUAGE plpgsql
SECURITY DEFINER -- Chạy dưới quyền bypass RLS để insert
AS $$
DECLARE
    current_balance INT;
BEGIN
    -- Tính số dư hiện tại
    SELECT balance INTO current_balance 
    FROM user_balances 
    WHERE user_id = p_user_id;

    IF current_balance IS NULL THEN
        current_balance := 0;
    END IF;

    -- Kiểm tra nếu đủ tiền
    IF current_balance >= p_amount THEN
        -- Insert một dòng reservation âm
        INSERT INTO credit_ledger (user_id, amount, reason, transaction_type, reference_id)
        VALUES (p_user_id, -p_amount, 'AI Request Reservation', 'reservation', p_request_id);
        RETURN TRUE;
    ELSE
        RETURN FALSE;
    END IF;
END;
$$;

-- 4. Hàm Commit Credits (Chốt tiền sau khi AI chạy xong)
-- Nhận vào số tiền thực tế sử dụng. Hoàn phần chênh lệch (nếu reserve dư)
CREATE OR REPLACE FUNCTION commit_credits(p_user_id UUID, p_amount_used INT, p_request_id UUID)
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
            VALUES (p_user_id, ABS(reserved_amount) - p_amount_used, 'Partial Refund (Over-reserved)', 'refund', p_request_id);
        END IF;

        -- Đánh dấu dòng cũ thành usage_commit để không bị xử lý lại
        UPDATE credit_ledger
        SET transaction_type = 'usage_commit'
        WHERE user_id = p_user_id 
          AND reference_id = p_request_id 
          AND transaction_type = 'reservation';
    END IF;
END;
$$;

-- 5. Hàm Refund Credits (Hoàn tiền 100% nếu gọi AI fail)
CREATE OR REPLACE FUNCTION refund_credits(p_user_id UUID, p_request_id UUID)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
AS $$
DECLARE
    reserved_amount INT;
BEGIN
    -- Tìm số tiền đã reserve
    SELECT amount INTO reserved_amount
    FROM credit_ledger
    WHERE user_id = p_user_id 
      AND reference_id = p_request_id 
      AND transaction_type = 'reservation'
    LIMIT 1;

    -- Nếu có reserve, cộng trả đúng số đó
    IF reserved_amount IS NOT NULL THEN
        INSERT INTO credit_ledger (user_id, amount, reason, transaction_type, reference_id)
        VALUES (p_user_id, ABS(reserved_amount), 'Full Refund (AI Failed)', 'refund', p_request_id);
        
        -- Chuyển trạng thái để tránh duplicate refund
        UPDATE credit_ledger
        SET transaction_type = 'refund'
        WHERE user_id = p_user_id 
          AND reference_id = p_request_id 
          AND transaction_type = 'reservation';
    END IF;
END;
$$;
