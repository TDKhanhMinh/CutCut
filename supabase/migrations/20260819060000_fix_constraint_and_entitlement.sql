-- Fix 1: Bổ sung 'trial/bonus' vào CHECK constraint của transaction_type
-- Constraint cũ chỉ cho phép: 'purchase', 'reservation', 'usage_commit', 'refund', 'adjustment'
-- Migration Task 45 cần chèn 'trial/bonus' nên constraint cũ bị vi phạm

ALTER TABLE credit_ledger
DROP CONSTRAINT IF EXISTS credit_ledger_transaction_type_check;

ALTER TABLE credit_ledger
ADD CONSTRAINT credit_ledger_transaction_type_check
  CHECK (transaction_type IN (
    'purchase',
    'reservation',
    'usage_commit',
    'refund',
    'adjustment',
    'trial/bonus'
  ));
