-- Add picture_url column to preparations table
ALTER TABLE preparations ADD COLUMN picture_url VARCHAR(500);

-- Set a default empty string for existing records
UPDATE preparations SET picture_url = '' WHERE picture_url IS NULL;
