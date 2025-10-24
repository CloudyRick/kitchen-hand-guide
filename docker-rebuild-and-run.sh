#!/bin/bash

# Kitchen Hand Guide - Docker Rebuild and Run Script
# This script rebuilds the Docker image and runs the container

set -e  # Exit on any error

echo "🐳 Kitchen Hand Guide - Docker Rebuild and Run"
echo "=============================================="

# Check if .env file exists
if [ ! -f ".env" ]; then
    echo "❌ Error: .env file not found!"
    echo "Please create a .env file with your configuration."
    exit 1
fi

# Stop and remove existing container if it exists
echo "🛑 Stopping and removing existing container (if any)..."
docker stop kitchen_hand_app 2>/dev/null || true
docker rm kitchen_hand_app 2>/dev/null || true

# Build the Docker image
echo "🔨 Building Docker image..."
DOCKER_BUILDKIT=1 docker build -t kitchen-hand-guide_app .

if [ $? -eq 0 ]; then
    echo "✅ Docker image built successfully!"
else
    echo "❌ Docker build failed!"
    exit 1
fi

# Run the container
echo "🚀 Starting container..."
docker run -d \
    --name kitchen_hand_app \
    --network kitchen-net \
    -p 8080:8080 \
    --env-file .env \
    kitchen-hand-guide_app

if [ $? -eq 0 ]; then
    echo "✅ Container started successfully!"
    
    # Clean up old Docker images (keep only the latest 2 versions)
    echo "🧹 Cleaning up old Docker images..."
    docker image prune -f --filter "dangling=true" 2>/dev/null || true
    
    # Remove old versions of kitchen-hand-guide_app (keep latest 2)
    OLD_IMAGES=$(docker images kitchen-hand-guide_app --format "table {{.ID}}\t{{.CreatedAt}}" | tail -n +2 | head -n -2 | awk '{print $1}')
    if [ ! -z "$OLD_IMAGES" ]; then
        echo "🗑️  Removing old kitchen-hand-guide_app images..."
        echo "$OLD_IMAGES" | xargs docker rmi -f 2>/dev/null || true
    fi
    
    echo ""
    echo "📋 Container Details:"
    echo "   Name: kitchen_hand_app"
    echo "   Image: kitchen-hand-guide_app"
    echo "   Network: kitchen-net"
    echo "   Environment: .env file"
    echo ""
    echo "🌐 Application should be available at:"
    echo "   http://127.0.0.1:8080"
    echo ""
    echo "📊 Useful commands:"
    echo "   View logs:     docker logs kitchen_hand_app"
    echo "   Follow logs:   docker logs -f kitchen_hand_app"
    echo "   Stop container: docker stop kitchen_hand_app"
    echo "   Remove container: docker rm kitchen_hand_app"
    echo ""
    echo "🎉 Kitchen Hand Guide is now running in Docker!"
else
    echo "❌ Failed to start container!"
    exit 1
fi
