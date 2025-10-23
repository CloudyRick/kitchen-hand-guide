-- Add preparation steps and users tables
-- Run this with: psql $DATABASE_URL -f migrations/002_add_users_and_prep_steps.sql

-- Preparation Steps table
CREATE TABLE IF NOT EXISTS preparation_steps (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    preparation_id UUID NOT NULL REFERENCES preparations(id) ON DELETE CASCADE,
    step_number INTEGER NOT NULL,
    description TEXT NOT NULL,
    picture_url VARCHAR(500),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(preparation_id, step_number)
);

CREATE INDEX IF NOT EXISTS idx_prep_steps_preparation ON preparation_steps(preparation_id);

DROP TRIGGER IF EXISTS update_preparation_steps_updated_at ON preparation_steps;
CREATE TRIGGER update_preparation_steps_updated_at BEFORE UPDATE ON preparation_steps
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Users table
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    username VARCHAR(50) NOT NULL UNIQUE,
    email VARCHAR(255) NOT NULL UNIQUE,
    password_hash VARCHAR(255) NOT NULL,
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);

DROP TRIGGER IF EXISTS update_users_updated_at ON users;
CREATE TRIGGER update_users_updated_at BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Insert default admin user (password: admin123 - CHANGE THIS IN PRODUCTION!)
-- Password hash is for 'admin123' using bcrypt
INSERT INTO users (username, email, password_hash) VALUES
    ('admin', 'admin@kitchen-hand.local', '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewY.5Q8J8z4bOhyS')
ON CONFLICT (username) DO NOTHING;
