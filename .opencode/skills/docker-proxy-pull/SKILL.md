---
name: docker-proxy-pull
description: Use proxy to download Docker images and import to local Docker. Trigger when user wants to pull Docker images through a proxy, download images from Docker Hub using proxy, or transfer images from podman to docker environment.
---

# Docker Proxy Pull

This skill helps you download Docker images through an HTTP proxy using podman, then export and import them into the local Docker daemon. This is useful when the local Docker daemon cannot directly access the internet or Docker Hub.

## When to use

- Docker daemon cannot directly access the internet
- Need to pull images through an HTTP/HTTPS proxy
- Podman has proxy access but Docker does not
- Need to transfer images from podman to docker environment
- User specifies a custom proxy URL

## Prerequisites

- podman installed and accessible in PATH
- docker installed and accessible in PATH
- Proxy server accessible (default: `http://192.168.5.244:10808`)

## Configuration

### Default Proxy
By default, the skill uses proxy: `http://192.168.5.244:10808`

### Custom Proxy
You can specify a custom proxy URL in two ways:

1. **Environment Variable** (recommended):
   ```bash
   export HTTPS_PROXY=http://your-proxy:port
   bash scripts/docker_proxy_pull.sh <image_name>
   ```

2. **Inline with command**:
   ```bash
   HTTPS_PROXY=http://your-proxy:port bash scripts/docker_proxy_pull.sh <image_name>
   ```

## Workflow

1. **Pull** the image using podman with HTTPS_PROXY
2. **Save** the image to a tar file
3. **Load** the image into Docker

## Usage

### Basic Usage (with default proxy)

```bash
bash scripts/docker_proxy_pull.sh docker.io/ubuntu:24.04
```

### With Custom Proxy

```bash
# Set proxy via environment variable
export HTTPS_PROXY=http://proxy.example.com:8080
bash scripts/docker_proxy_pull.sh docker.io/nginx:latest

# Or inline
HTTPS_PROXY=http://proxy.example.com:8080 bash scripts/docker_proxy_pull.sh docker.io/redis:alpine
```

### With Custom Output Path

```bash
bash scripts/docker_proxy_pull.sh docker.io/ubuntu:24.04 /custom/path/ubuntu.tar
```

## Examples

| Command | Description |
|---------|-------------|
| `bash scripts/docker_proxy_pull.sh docker.io/ubuntu:24.04` | Pull ubuntu with default proxy |
| `HTTPS_PROXY=http://proxy.company.com:8080 bash scripts/docker_proxy_pull.sh docker.io/nginx:latest` | Pull nginx with custom proxy |
| `export HTTPS_PROXY=http://192.168.1.100:3128; bash scripts/docker_proxy_pull.sh docker.io/redis:alpine` | Pull redis with custom proxy |

## Script Reference

Read `scripts/docker_proxy_pull.sh` for the implementation details.

### Environment Variables

- `HTTPS_PROXY` - HTTPS proxy URL (default: `http://192.168.5.244:10808`)
- `HTTP_PROXY` - HTTP proxy URL (used if `HTTPS_PROXY` is not set)

### Arguments

1. `image_name` (required) - Docker image to pull (e.g., `docker.io/ubuntu:24.04`)
2. `output_tar_path` (optional) - Custom path for temporary tar file
