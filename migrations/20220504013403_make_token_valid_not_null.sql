BEGIN;

UPDATE subscription_tokens
    SET is_valid = false
    WHERE is_valid IS NULL;

ALTER TABLE subscription_tokens
    ALTER COLUMN is_valid
        SET NOT NULL;

COMMIT;
