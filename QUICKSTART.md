# Quick Start Guide

Get the Kitchen Hand Guide running in 5 minutes!

## Step 1: Prerequisites

Ensure you have:
- Rust installed: `rustc --version`
- PostgreSQL installed and running: `pg_isready`

If not, install them:
- Rust: https://rustup.rs/
- PostgreSQL: https://www.postgresql.org/download/

## Step 2: Database Setup

```bash
# Create database
createdb kitchen_hand_guide

# Or using psql
psql -U postgres -c "CREATE DATABASE kitchen_hand_guide;"

# Run schema
psql -U postgres -d kitchen_hand_guide -f schema.sql
```

## Step 3: Configure

The `.env` file is already created with defaults. If needed, update your database credentials:

```bash
# Edit .env file
DATABASE_URL=postgres://your_username:your_password@localhost/kitchen_hand_guide
```

## Step 4: Run

```bash
# Install dependencies and run
cargo run
```

Wait for compilation (first time takes 2-5 minutes).

## Step 5: Open Browser

Navigate to:
```
http://127.0.0.1:8080
```

## You're Done!

Try adding your first product:
1. Click "Add New Product"
2. Fill in the form
3. Upload an image
4. Click "Add Product"

## Troubleshooting

### Database Error?
- Check PostgreSQL is running: `pg_isready`
- Verify database exists: `psql -U postgres -l | grep kitchen_hand_guide`

### Port 8080 in use?
- Change PORT in `.env` to 8081 or another available port

### Compilation errors?
- Update Rust: `rustup update`
- Clean and rebuild: `cargo clean && cargo build`

## Next Steps

- Read the full [README.md](README.md) for detailed documentation
- Customize templates in `templates/` folder
- Modify styles in `static/styles.css`
- Add more features in `src/handlers.rs`

Enjoy your Kitchen Hand Guide!
