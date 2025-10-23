# Kitchen Hand Training Guide

A complete web application built with Rust, Actix Web, Askama templates (server-side rendering), and PostgreSQL. This application serves as a dynamic training guide for new kitchen hands, presenting information about products, deliveries, storage locations, and responsibilities.

## Features

- **Product Management**: Add, view, and manage kitchen products with images and descriptions
- **Image Upload**: Support for uploading product images (JPG, PNG, WEBP)
- **Server-Side Rendering**: Fast page loads using Askama templates
- **Responsive Design**: Mobile-friendly UI using Bootstrap 5
- **PostgreSQL Database**: Reliable data storage with UUID-based primary keys
- **Clean Architecture**: Organized modules for maintainability

## Tech Stack

- **Backend**: Actix Web (Rust web framework)
- **Database**: PostgreSQL with SQLx (async SQL toolkit)
- **Templates**: Askama (type-safe compiled templates)
- **Frontend**: Bootstrap 5 (CSS framework via CDN)
- **Language**: Rust 2021 edition

## Prerequisites

Before you begin, ensure you have the following installed:

- **Rust** (1.70 or later): [Install Rust](https://rustup.rs/)
- **PostgreSQL** (14 or later): [Install PostgreSQL](https://www.postgresql.org/download/)
- **Cargo** (comes with Rust)

## Installation & Setup

### 1. Clone or Download the Project

```bash
cd kitchen-hand-guide
```

### 2. Set Up PostgreSQL Database

#### Create the database:

```bash
# Login to PostgreSQL
psql -U postgres

# Create database
CREATE DATABASE kitchen_hand_guide;

# Exit psql
\q
```

#### Run the schema:

```bash
psql -U postgres -d kitchen_hand_guide -f schema.sql
```

This will:
- Create the `products` table
- Add necessary indexes
- Set up triggers for auto-updating timestamps
- Insert sample data (optional)

### 3. Configure Environment Variables

Copy the example environment file and edit it:

```bash
cp .env.example .env
```

Edit `.env` with your database credentials:

```bash
# Database Configuration
DATABASE_URL=postgres://your_username:your_password@localhost/kitchen_hand_guide

# Server Configuration
HOST=127.0.0.1
PORT=8080

# Upload Configuration
UPLOAD_DIR=./static/uploads
MAX_FILE_SIZE=5242880
```

**Important**: Replace `your_username` and `your_password` with your actual PostgreSQL credentials.

### 4. Build and Run

#### Development mode (with logging):

```bash
RUST_LOG=info cargo run
```

#### Production build:

```bash
cargo build --release
./target/release/kitchen-hand-guide
```

### 5. Access the Application

Open your browser and navigate to:

```
http://127.0.0.1:8080
```

## Usage Guide

### Adding a New Product

1. Click **"Add New Product"** button on the homepage
2. Fill in the form:
   - **Supplier Name**: The company providing the product
   - **Product Name**: Name of the product
   - **Storage Location**: Where to store it (e.g., "Cold Room A - Shelf 2")
   - **Product Image**: Upload a clear photo (max 5MB)
   - **Description**: Storage instructions, temperature, shelf life, etc.
3. Click **"Add Product"**
4. You'll be redirected to the product detail page

### Viewing Products

- **Homepage**: Shows all products in a card grid layout
- **Product Detail**: Click "View Details" on any product card

## Project Structure

```
kitchen-hand-guide/
├── Cargo.toml              # Rust dependencies and project metadata
├── .env.example            # Environment variables template
├── .gitignore              # Git ignore rules
├── schema.sql              # Database schema and migrations
├── README.md               # This file
├── src/
│   ├── main.rs             # Application entry point and server setup
│   ├── models.rs           # Database models and structs
│   ├── handlers.rs         # Route handlers and business logic
│   ├── db.rs               # Database connection pool setup
│   └── utils.rs            # Utility functions (file upload, etc.)
├── templates/              # Askama HTML templates
│   ├── base.html           # Base layout template
│   ├── index.html          # Homepage (product list)
│   ├── product_new.html    # Add product form
│   └── product_detail.html # Product detail view
└── static/                 # Static assets
    ├── styles.css          # Custom CSS
    └── uploads/            # Uploaded product images
        └── .gitkeep        # Keeps directory in git
```

## Routes

| Method | Route            | Description                      |
|--------|------------------|----------------------------------|
| GET    | `/`              | Homepage with list of products   |
| GET    | `/product/new`   | Show form to add new product     |
| POST   | `/product`       | Handle form submission           |
| GET    | `/product/{id}`  | View single product details      |
| GET    | `/static/*`      | Serve static files (CSS, images) |

## Database Schema

### Products Table

```sql
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
```

## Development

### Running Tests

```bash
cargo test
```

### Code Formatting

```bash
cargo fmt
```

### Linting

```bash
cargo clippy
```

### Hot Reload (Development)

Install `cargo-watch`:

```bash
cargo install cargo-watch
```

Run with auto-reload:

```bash
cargo watch -x run
```

## Troubleshooting

### Database Connection Issues

**Error**: "Failed to create database pool"

**Solution**:
- Verify PostgreSQL is running: `pg_isready`
- Check your `DATABASE_URL` in `.env`
- Ensure database exists: `psql -U postgres -l`

### File Upload Issues

**Error**: "Failed to create upload directory"

**Solution**:
- Ensure `./static/uploads` directory exists
- Check write permissions: `chmod 755 static/uploads`

### Port Already in Use

**Error**: "Address already in use"

**Solution**:
- Change the `PORT` in `.env` to a different value (e.g., 8081)
- Or kill the process using port 8080: `lsof -ti:8080 | xargs kill`

## Configuration

### Maximum File Upload Size

Default: 5MB (5242880 bytes)

To change, edit `MAX_FILE_SIZE` in `.env` and update the multipart limit in `src/handlers.rs`:

```rust
#[multipart(limit = "10 MB")]  // Change this value
picture: TempFile,
```

### Supported Image Formats

- JPEG (.jpg, .jpeg)
- PNG (.png)
- WebP (.webp)

## Security Considerations

- File uploads are validated by extension
- Filenames are sanitized and UUIDs are used
- SQL injection is prevented by using SQLx parameterized queries
- File size limits prevent DOS attacks

## Future Enhancements

- [ ] User authentication and authorization
- [ ] Product editing and deletion
- [ ] Search and filter functionality
- [ ] Categories and tags for products
- [ ] Barcode scanning support
- [ ] Export data to PDF/Excel
- [ ] Multi-language support
- [ ] Email notifications for expiring products

## Contributing

1. Fork the repository
2. Create a feature branch: `git checkout -b feature-name`
3. Commit your changes: `git commit -am 'Add feature'`
4. Push to the branch: `git push origin feature-name`
5. Submit a pull request

## License

This project is provided as-is for educational and commercial use.

## Support

For issues and questions:
- Check the [Troubleshooting](#troubleshooting) section
- Review the code comments in source files
- Consult Actix Web documentation: https://actix.rs
- SQLx documentation: https://github.com/launchbadge/sqlx

## Credits

Built with:
- [Actix Web](https://actix.rs/) - Web framework
- [SQLx](https://github.com/launchbadge/sqlx) - Async SQL toolkit
- [Askama](https://github.com/djc/askama) - Type-safe templates
- [Bootstrap](https://getbootstrap.com/) - CSS framework
