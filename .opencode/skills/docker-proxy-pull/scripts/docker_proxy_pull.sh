#!/bin/bash

# Docker Proxy Pull Script
# Pulls Docker images through proxy using podman, then imports to Docker
#
# Usage:
#   docker_proxy_pull.sh <image_name> [output_tar_path]
#
# Environment Variables:
#   HTTPS_PROXY - Proxy URL (default: http://192.168.5.244:10808)
#   HTTP_PROXY  - Alternative proxy URL (used if HTTPS_PROXY not set)
#
# Examples:
#   docker_proxy_pull.sh docker.io/ubuntu:24.04
#   HTTPS_PROXY=http://proxy.example.com:8080 docker_proxy_pull.sh docker.io/nginx:latest
#

set -e

# Configuration - use environment variable or default
DEFAULT_PROXY="http://192.168.5.244:10808"
PROXY_URL="${HTTPS_PROXY:-${HTTP_PROXY:-$DEFAULT_PROXY}}"

# Parse arguments
IMAGE_NAME="$1"
OUTPUT_TAR="${2:-}"

# Validate input
if [ -z "$IMAGE_NAME" ]; then
    echo "Usage: $0 <image_name> [output_tar_path]"
    echo ""
    echo "Environment Variables:"
    echo "  HTTPS_PROXY  Proxy URL (default: $DEFAULT_PROXY)"
    echo "  HTTP_PROXY   Alternative proxy URL"
    echo ""
    echo "Examples:"
    echo "  $0 docker.io/ubuntu:24.04"
    echo "  HTTPS_PROXY=http://proxy.example.com:8080 $0 docker.io/nginx:latest"
    exit 1
fi

# Generate output filename if not provided
if [ -z "$OUTPUT_TAR" ]; then
    # Sanitize image name for filename
    SAFE_NAME=$(echo "$IMAGE_NAME" | tr '/:' '_')
    OUTPUT_TAR="/tmp/${SAFE_NAME}.tar"
fi

echo "=== Docker Proxy Pull ==="
echo "Image: $IMAGE_NAME"
echo "Proxy: $PROXY_URL"
echo "Output: $OUTPUT_TAR"
echo ""

# Step 1: Pull image using podman with proxy
echo "Step 1: Pulling image via podman with proxy..."
if ! HTTPS_PROXY="$PROXY_URL" podman pull "$IMAGE_NAME"; then
    echo "ERROR: Failed to pull image via podman"
    exit 1
fi
echo "✓ Image pulled successfully"
echo ""

# Step 2: Save image to tar
echo "Step 2: Saving image to tar file..."
if ! podman save "$IMAGE_NAME" -o "$OUTPUT_TAR"; then
    echo "ERROR: Failed to save image to tar"
    exit 1
fi
echo "✓ Image saved to $OUTPUT_TAR"
echo ""

# Step 3: Load image into Docker
echo "Step 3: Loading image into Docker..."
if ! docker load -i "$OUTPUT_TAR"; then
    echo "ERROR: Failed to load image into Docker"
    exit 1
fi
echo "✓ Image loaded into Docker successfully"
echo ""

# Step 4: Cleanup
echo "Step 4: Cleaning up temporary files..."
rm -f "$OUTPUT_TAR"
echo "✓ Cleanup complete"
echo ""

# Verify
echo "=== Verification ==="
echo "Checking if image is available in Docker..."
# Extract image name without registry for docker images check
IMAGE_SHORT=$(echo "$IMAGE_NAME" | sed 's|^docker.io/||' | sed 's|^library/||')
docker images | grep -E "$(echo "$IMAGE_SHORT" | cut -d: -f1)" || true

echo ""
echo "=== Complete ==="
echo "Image '$IMAGE_NAME' is now available in Docker via proxy ($PROXY_URL)!"
