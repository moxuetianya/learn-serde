---
name: environment-setup
description: Provides standard environment setup commands for sudo execution and network proxy configuration. Use ONLY when opencode needs to run sudo commands or configure HTTP/HTTPS proxies for network access.
---

# Environment Setup

Standard operating environment configuration for opencode, including sudo command execution through `ssh-askpass` and network proxy settings.

## Sudo Command Execution

When a command requires `sudo`, always use the `ssh-askpass` helper to provide the password non-interactively:

```bash
SUDO_ASKPASS=/usr/bin/ssh-askpass sudo -A <command>
```

This avoids interactive password prompts and works with opencode's non-TTY execution environment.

### Usage

```bash
SUDO_ASKPASS=/usr/bin/ssh-askpass sudo -A apt-get update
SUDO_ASKPASS=/usr/bin/ssh-askpass sudo -A systemctl restart nginx
SUDO_ASKPASS=/usr/bin/ssh-askpass sudo -A docker ps
```

## Network Proxy Configuration

When network access to external resources (e.g. GitHub, npm, Docker Hub) is blocked and requires a proxy, export the following environment variables before running network commands:

```bash
export http_proxy=http://192.168.5.244:10808
export HTTP_PROXY=http://192.168.5.244:10808
export https_proxy=http://192.168.5.244:10808
export HTTPS_PROXY=http://192.168.5.244:10808
export all_proxy=socks5://192.168.5.244:10808
export no_proxy=localhost,127.0.0.1,::1
```

**Note:** The proxy server address may vary by environment. The default is `192.168.5.244:10808`. Adjust the IP and port as needed.

### When to use

- `git clone` / `git pull` from GitHub fails with connection timeout
- `npm install` / `pip install` / `cargo build` cannot reach registries
- Any HTTP/HTTPS operation that times out or is refused
- Docker/podman pull operations that fail to reach registries

### Usage Example

```bash
export https_proxy=http://192.168.5.244:10808
curl -v https://github.com
```

## Global AGENTS.md Configuration

The global `AGENTS.md` file that provides persistent instructions to opencode is located at:

```
~/.config/opencode/AGENTS.md
```

Use this file to define instructions that apply across all projects, such as the sudo and proxy configurations documented in this skill.

### Example AGENTS.md content

```markdown
# 如何调用sudo
SUDO_ASKPASS=/usr/bin/ssh-askpass sudo -A <command>

# 使用网络代理
export http_proxy=http://192.168.5.244:10808
export HTTP_PROXY=http://192.168.5.244:10808
export https_proxy=http://192.168.5.244:10808
export HTTPS_PROXY=http://192.168.5.244:10808
export all_proxy=socks5://192.168.5.244:10808
export no_proxy=localhost,127.0.0.1,::1
```
