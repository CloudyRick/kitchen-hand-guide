# Kitchen Hand Guide - Production Deployment Guide

This guide covers deploying the Kitchen Hand Guide application to production using various platforms and configurations.

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Environment Configuration](#environment-configuration)
3. [Database Setup](#database-setup)
4. [AWS S3 Configuration](#aws-s3-configuration)
5. [Deployment Options](#deployment-options)
   - [Docker Deployment](#docker-deployment)
   - [VPS Deployment](#vps-deployment)
   - [Railway Deployment](#railway-deployment)
   - [Render Deployment](#render-deployment)
6. [Security Checklist](#security-checklist)
7. [Monitoring and Maintenance](#monitoring-and-maintenance)

---

## Prerequisites

### Required Software
- **Rust** 1.70+ (for building)
- **PostgreSQL** 14+
- **Docker** (optional, for containerized deployment)
- **AWS Account** (if using S3 for image storage)

### System Requirements
- **RAM**: Minimum 512MB, recommended 1GB+
- **Storage**: Minimum 1GB for application + database
- **CPU**: 1 vCPU minimum

---

## Environment Configuration

### Production .env File

Create a `.env.production` file with the following configuration:

```bash
# Database
DATABASE_URL=postgresql://username:password@host:5432/kitchen_hand_guide

# Server
HOST=0.0.0.0
PORT=8080

# JWT Authentication
JWT_SECRET=your-super-secure-random-string-min-32-chars-change-this-in-production
JWT_EXPIRATION_HOURS=24

# File Storage
UPLOAD_DIR=./static/uploads

# AWS S3 (Optional - for cloud storage)
S3_ENABLED=true
S3_BUCKET_NAME=kitchen-hand-guide
AWS_REGION=us-east-1
# AWS credentials should be set via IAM role or environment variables:
# AWS_ACCESS_KEY_ID=your-access-key
# AWS_SECRET_ACCESS_KEY=your-secret-key

# Logging
RUST_LOG=info
```

### Generate Secure JWT Secret

Generate a secure random string for JWT_SECRET:

```bash
# On Linux/Mac
openssl rand -base64 48

# Or using Python
python3 -c "import secrets; print(secrets.token_urlsafe(48))"
```

---

## Database Setup

### 1. Create Production Database

```bash
# Connect to PostgreSQL
psql -U postgres

# Create database and user
CREATE DATABASE kitchen_hand_guide;
CREATE USER kitchen_user WITH ENCRYPTED PASSWORD '!';
GRANT ALL PRIVILEGES ON DATABASE kitchen_hand_guide TO kitchen_user;

# Enable UUID extension
\c kitchen_hand_guide
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
```

### 2. Run Migrations

```bash
# Run all migrations in order
psql "postgresql://kitchen_user:password@localhost/kitchen_hand_guide" -f migrations/001_initial_schema.sql
psql "postgresql://kitchen_user:password@localhost/kitchen_hand_guide" -f migrations/002_add_users_and_prep_steps.sql
```

### 3. Create Admin User

The migration automatically creates a default admin user:
- **Username**: `admin`
- **Password**: `admin123`

**IMPORTANT**: Change this password immediately after deployment!

To create a new admin user programmatically:

```bash
# Hash a password using bcrypt (requires bcrypt-cli or Python)
python3 -c "import bcrypt; print(bcrypt.hashpw(b'your-password', bcrypt.gensalt()).decode())"

# Insert into database
psql "postgresql://kitchen_user:password@localhost/kitchen_hand_guide" -c "
INSERT INTO users (username, email, password_hash)
VALUES ('youradmin', 'admin@example.com', 'bcrypt-hash-here');"
```

---

## AWS S3 Configuration

### 1. Create S3 Bucket

```bash
# Using AWS CLI
aws s3 mb s3://kitchen-hand-guide --region us-east-1

# Configure bucket for public read access (images only)
aws s3api put-bucket-cors --bucket kitchen-hand-guide --cors-configuration file://cors.json
```

### 2. CORS Configuration (cors.json)

```json
{
  "CORSRules": [
    {
      "AllowedOrigins": ["https://yourdomain.com"],
      "AllowedMethods": ["GET", "HEAD"],
      "AllowedHeaders": ["*"],
      "ExposeHeaders": ["ETag"],
      "MaxAgeSeconds": 3000
    }
  ]
}
```

### 3. IAM Policy

Create an IAM user or role with this policy:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "s3:PutObject",
        "s3:GetObject",
        "s3:DeleteObject"
      ],
      "Resource": "arn:aws:s3:::kitchen-hand-guide/*"
    },
    {
      "Effect": "Allow",
      "Action": "s3:ListBucket",
      "Resource": "arn:aws:s3:::kitchen-hand-guide"
    }
  ]
}
```

### 4. Make Bucket Public for Images

```bash
aws s3api put-bucket-policy --bucket kitchen-hand-guide --policy '{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Sid": "PublicReadGetObject",
      "Effect": "Allow",
      "Principal": "*",
      "Action": "s3:GetObject",
      "Resource": "arn:aws:s3:::kitchen-hand-guide/*"
    }
  ]
}'
```

---

## Deployment Options

### Docker Deployment

#### 1. Create Dockerfile

```dockerfile
# Multi-stage build for smaller image
FROM rust:1.75 as builder

WORKDIR /app
COPY . .

# Build in release mode
RUN cargo build --release

# Runtime image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libpq5 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/kitchen-hand-guide .
COPY --from=builder /app/templates ./templates
COPY --from=builder /app/static ./static

# Create upload directory
RUN mkdir -p ./static/uploads

# Expose port
EXPOSE 8080

# Run the application
CMD ["./kitchen-hand-guide"]
```

#### 2. Create docker-compose.yml

```yaml
version: '3.8'

services:
  app:
    build: .
    ports:
      - "8080:8080"
    environment:
      - DATABASE_URL=postgresql://kitchen_user:password@db:5432/kitchen_hand_guide
      - HOST=0.0.0.0
      - PORT=8080
      - JWT_SECRET=${JWT_SECRET}
      - S3_ENABLED=true
      - S3_BUCKET_NAME=${S3_BUCKET_NAME}
      - AWS_REGION=${AWS_REGION}
      - AWS_ACCESS_KEY_ID=${AWS_ACCESS_KEY_ID}
      - AWS_SECRET_ACCESS_KEY=${AWS_SECRET_ACCESS_KEY}
      - RUST_LOG=info
    depends_on:
      - db
    restart: unless-stopped

  db:
    image: postgres:15-alpine
    environment:
      - POSTGRES_DB=kitchen_hand_guide
      - POSTGRES_USER=kitchen_user
      - POSTGRES_PASSWORD=${DB_PASSWORD}
    volumes:
      - postgres_data:/var/lib/postgresql/data
      - ./migrations:/docker-entrypoint-initdb.d
    restart: unless-stopped

volumes:
  postgres_data:
```

#### 3. Deploy with Docker

```bash
# Build and start
docker-compose up -d

# View logs
docker-compose logs -f app

# Stop
docker-compose down
```

---

### VPS Deployment (Ubuntu/Debian)

#### 1. Install Dependencies

```bash
# Update system
sudo apt update && sudo apt upgrade -y

# Install PostgreSQL
sudo apt install -y postgresql postgresql-contrib

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Install build dependencies
sudo apt install -y build-essential libssl-dev pkg-config libpq-dev
```

#### 2. Setup PostgreSQL

```bash
# Switch to postgres user
sudo -u postgres psql

# Create database and user (see Database Setup section above)
```

#### 3. Build Application

```bash
# Clone repository
git clone <your-repo> kitchen-hand-guide
cd kitchen-hand-guide

# Build in release mode
cargo build --release

# Copy binary to /usr/local/bin
sudo cp target/release/kitchen-hand-guide /usr/local/bin/
sudo chmod +x /usr/local/bin/kitchen-hand-guide
```

#### 4. Create Systemd Service

Create `/etc/systemd/system/kitchen-hand-guide.service`:

```ini
[Unit]
Description=Kitchen Hand Guide Web Application
After=network.target postgresql.service

[Service]
Type=simple
User=www-data
WorkingDirectory=/var/www/kitchen-hand-guide
EnvironmentFile=/var/www/kitchen-hand-guide/.env.production
ExecStart=/usr/local/bin/kitchen-hand-guide
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

#### 5. Setup Application Directory

```bash
# Create directory
sudo mkdir -p /var/www/kitchen-hand-guide
sudo cp -r templates static /var/www/kitchen-hand-guide/
sudo cp .env.production /var/www/kitchen-hand-guide/
sudo mkdir -p /var/www/kitchen-hand-guide/static/uploads
sudo chown -R www-data:www-data /var/www/kitchen-hand-guide
```

#### 6. Start Service

```bash
# Reload systemd
sudo systemctl daemon-reload

# Enable and start service
sudo systemctl enable kitchen-hand-guide
sudo systemctl start kitchen-hand-guide

# Check status
sudo systemctl status kitchen-hand-guide

# View logs
sudo journalctl -u kitchen-hand-guide -f
```

#### 7. Setup Nginx Reverse Proxy

Create `/etc/nginx/sites-available/kitchen-hand-guide`:

```nginx
server {
    listen 80;
    server_name yourdomain.com;

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_cache_bypass $http_upgrade;

        # Increase upload size limit
        client_max_body_size 10M;
    }

    location /static/ {
        alias /var/www/kitchen-hand-guide/static/;
        expires 30d;
        add_header Cache-Control "public, immutable";
    }
}
```

Enable site and setup SSL:

```bash
# Enable site
sudo ln -s /etc/nginx/sites-available/kitchen-hand-guide /etc/nginx/sites-enabled/
sudo nginx -t
sudo systemctl reload nginx

# Install Certbot for SSL
sudo apt install -y certbot python3-certbot-nginx
sudo certbot --nginx -d yourdomain.com
```

---

### Railway Deployment

#### 1. Create railway.json

```json
{
  "$schema": "https://railway.app/railway.schema.json",
  "build": {
    "builder": "NIXPACKS"
  },
  "deploy": {
    "startCommand": "./target/release/kitchen-hand-guide",
    "restartPolicyType": "ON_FAILURE",
    "restartPolicyMaxRetries": 10
  }
}
```

#### 2. Create nixpacks.toml

```toml
[phases.setup]
nixPkgs = ["postgresql"]

[phases.build]
cmds = ["cargo build --release"]

[start]
cmd = "./target/release/kitchen-hand-guide"
```

#### 3. Deploy

```bash
# Install Railway CLI
npm install -g @railway/cli

# Login
railway login

# Initialize project
railway init

# Add PostgreSQL
railway add postgresql

# Set environment variables
railway variables set JWT_SECRET=your-secret
railway variables set S3_ENABLED=true
railway variables set S3_BUCKET_NAME=your-bucket

# Deploy
railway up
```

---

### Render Deployment

#### 1. Create render.yaml

```yaml
services:
  - type: web
    name: kitchen-hand-guide
    env: rust
    buildCommand: cargo build --release
    startCommand: ./target/release/kitchen-hand-guide
    envVars:
      - key: DATABASE_URL
        fromDatabase:
          name: kitchen-hand-db
          property: connectionString
      - key: JWT_SECRET
        generateValue: true
      - key: S3_ENABLED
        value: true
      - key: RUST_LOG
        value: info

databases:
  - name: kitchen-hand-db
    databaseName: kitchen_hand_guide
    user: kitchen_user
```

#### 2. Deploy

1. Connect your GitHub repository to Render
2. Create new Web Service
3. Select your repository
4. Render will auto-detect Rust and build
5. Add environment variables in Render dashboard
6. Deploy

---

## Security Checklist

### Before Going Live

- [ ] Change default admin password
- [ ] Generate strong JWT_SECRET (min 32 chars)
- [ ] Use strong database passwords
- [ ] Enable HTTPS/SSL certificates
- [ ] Configure CORS properly
- [ ] Set up firewall rules
- [ ] Enable rate limiting
- [ ] Review and restrict S3 bucket permissions
- [ ] Set appropriate file upload size limits
- [ ] Configure secure HTTP headers
- [ ] Enable database backups
- [ ] Set up monitoring and alerts

### Recommended Nginx Security Headers

Add to your Nginx configuration:

```nginx
add_header X-Frame-Options "SAMEORIGIN" always;
add_header X-Content-Type-Options "nosniff" always;
add_header X-XSS-Protection "1; mode=block" always;
add_header Referrer-Policy "no-referrer-when-downgrade" always;
add_header Content-Security-Policy "default-src 'self' https: data: 'unsafe-inline' 'unsafe-eval';" always;
```

---

## Monitoring and Maintenance

### Application Logs

```bash
# Systemd service logs
sudo journalctl -u kitchen-hand-guide -f

# Docker logs
docker-compose logs -f app

# Application logs (if logging to file)
tail -f /var/log/kitchen-hand-guide/app.log
```

### Database Backups

#### Automated Backup Script

Create `/usr/local/bin/backup-db.sh`:

```bash
#!/bin/bash
BACKUP_DIR="/var/backups/kitchen-hand-guide"
DATE=$(date +%Y%m%d_%H%M%S)
DB_NAME="kitchen_hand_guide"

mkdir -p $BACKUP_DIR

pg_dump -U kitchen_user $DB_NAME | gzip > $BACKUP_DIR/backup_$DATE.sql.gz

# Keep only last 7 days
find $BACKUP_DIR -name "backup_*.sql.gz" -mtime +7 -delete
```

Setup cron job:

```bash
# Run daily at 2 AM
0 2 * * * /usr/local/bin/backup-db.sh
```

### Health Checks

Create a simple health check endpoint or use:

```bash
# Check if service is running
curl -f http://localhost:8080/ || echo "Service down!"
```

### Resource Monitoring

```bash
# Check memory usage
free -h

# Check disk usage
df -h

# Check PostgreSQL connections
sudo -u postgres psql -c "SELECT count(*) FROM pg_stat_activity;"

# Monitor with htop
htop
```

---

## Troubleshooting

### Common Issues

#### Application won't start

```bash
# Check logs
sudo journalctl -u kitchen-hand-guide -n 50

# Check if port is in use
sudo lsof -i :8080

# Verify database connection
psql "$DATABASE_URL"
```

#### Database migration issues

```bash
# Check if tables exist
psql "$DATABASE_URL" -c "\dt"

# Re-run migrations
psql "$DATABASE_URL" -f migrations/001_initial_schema.sql
psql "$DATABASE_URL" -f migrations/002_add_users_and_prep_steps.sql
```

#### S3 upload failures

```bash
# Test AWS credentials
aws s3 ls s3://kitchen-hand-guide

# Check IAM permissions
aws iam get-user

# Verify environment variables
env | grep AWS
```

---

## Performance Optimization

### Database Indexing

Already included in migrations, but verify:

```sql
-- Check existing indexes
\di

-- Add indexes if needed
CREATE INDEX IF NOT EXISTS idx_products_created_at ON products(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_preparations_created_at ON preparations(created_at DESC);
```

### Nginx Caching

Add to Nginx config:

```nginx
proxy_cache_path /var/cache/nginx levels=1:2 keys_zone=app_cache:10m max_size=1g inactive=60m;

location / {
    proxy_cache app_cache;
    proxy_cache_valid 200 10m;
    proxy_cache_bypass $http_cache_control;
    # ... other proxy settings
}
```

---

## Support and Maintenance

For issues or questions:
1. Check application logs
2. Review this deployment guide
3. Consult the README.md for application-specific details
4. Check database connectivity and migrations

Regular maintenance tasks:
- Weekly: Review logs for errors
- Monthly: Update dependencies (`cargo update`)
- Quarterly: Security audit and dependency updates
- As needed: Database optimization and backups

---

## License

See LICENSE file in the repository.
