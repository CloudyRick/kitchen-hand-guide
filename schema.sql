-- Kitchen Hand Guide Database Schema
-- PostgreSQL

-- Create database (run this separately as superuser)
-- CREATE DATABASE kitchen_hand_guide;

-- Connect to the database
-- \c kitchen_hand_guide

-- Enable UUID extension
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Products table
CREATE TABLE products (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    supplier_name VARCHAR(255) NOT NULL,
    product_name VARCHAR(255) NOT NULL,
    location VARCHAR(255) NOT NULL,
    picture_url VARCHAR(500) NOT NULL,
    description TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Create index on supplier_name for faster queries
CREATE INDEX idx_products_supplier ON products(supplier_name);

-- Create index on location for faster queries
CREATE INDEX idx_products_location ON products(location);

-- Insert sample data (optional)
INSERT INTO products (supplier_name, product_name, location, picture_url, description) VALUES
    ('Fresh Farm Co.', 'Organic Tomatoes', 'Cold Room A - Shelf 2', '/static/uploads/placeholder.jpg', 'Fresh organic tomatoes. Store at 4°C. Check daily for spoilage. Shelf life: 5-7 days.'),
    ('Ocean Catch Ltd.', 'Atlantic Salmon Fillet', 'Freezer B - Drawer 3', '/static/uploads/placeholder.jpg', 'Premium Atlantic salmon. Keep frozen at -18°C. Thaw in refrigerator overnight before use. Use within 24 hours of thawing.'),
    ('Dairy Delights', 'Full Cream Milk', 'Refrigerator - Door Shelf', '/static/uploads/placeholder.jpg', 'Pasteurized full cream milk. Store at 4°C. Check use-by date daily. Once opened, use within 3 days.');

-- Function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Trigger to automatically update updated_at
CREATE TRIGGER update_products_updated_at BEFORE UPDATE ON products
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Preparations table
CREATE TABLE preparations (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    prep_type VARCHAR(50) NOT NULL CHECK (prep_type IN ('fruit', 'bread', 'veg', 'meat', 'seafood')),
    shift VARCHAR(50) NOT NULL CHECK (shift IN ('brekkie', 'lunch', 'both')),
    location VARCHAR(255) NOT NULL,
    steps TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Create index on prep_type for faster queries
CREATE INDEX idx_preparations_type ON preparations(prep_type);

-- Create index on shift for faster queries
CREATE INDEX idx_preparations_shift ON preparations(shift);

-- Insert sample data (optional)
INSERT INTO preparations (name, prep_type, shift, location, steps) VALUES
    ('Diced Tomatoes', 'veg', 'both', 'Prep Station 1', E'1. Wash tomatoes thoroughly under cold running water\n2. Remove the stem and core with a paring knife\n3. Cut tomatoes in half from top to bottom\n4. Place cut side down and slice into 1cm strips\n5. Rotate 90 degrees and dice into 1cm cubes\n6. Store in airtight container in cold room\n7. Label with date and time - use within 24 hours'),
    ('Bread Roll Portioning', 'bread', 'brekkie', 'Bread Station', E'1. Check bread delivery and verify freshness\n2. Count required portions for morning service\n3. Place rolls on clean tray lined with parchment paper\n4. Cover with clean tea towel to prevent drying\n5. Store at room temperature away from heat\n6. Warm in oven at 180°C for 3-4 minutes before service\n7. Serve immediately while warm'),
    ('Salmon Portioning', 'seafood', 'lunch', 'Fish Prep Area', E'1. Remove salmon from cold storage (must be 4°C or below)\n2. Ensure cutting board and knife are sanitized\n3. Remove pin bones using fish tweezers\n4. Pat dry with paper towel\n5. Cut into 180g portions using sharp filleting knife\n6. Check for any remaining bones\n7. Place portions on tray lined with parchment\n8. Cover with plastic wrap and return to cold storage\n9. Label with prep date and use-by date (24 hours)\n10. Wash hands and sanitize work area immediately after');

-- Trigger to automatically update updated_at for preparations
CREATE TRIGGER update_preparations_updated_at BEFORE UPDATE ON preparations
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

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

-- Create index on preparation_id for faster queries
CREATE INDEX idx_prep_steps_preparation ON preparation_steps(preparation_id);

-- Trigger to automatically update updated_at for preparation steps
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

-- Create index on username for faster login queries
CREATE INDEX idx_users_username ON users(username);
CREATE INDEX idx_users_email ON users(email);

-- Trigger to automatically update updated_at for users
CREATE TRIGGER update_users_updated_at BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Insert default admin user (password: admin123 - CHANGE THIS IN PRODUCTION!)
-- Password hash is for 'admin123' using bcrypt
INSERT INTO users (username, email, password_hash) VALUES
    ('admin', 'admin@kitchen-hand.local', '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewY.5Q8J8z4bOhyS')
ON CONFLICT (username) DO NOTHING;
