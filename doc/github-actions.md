# GitHub Actions Configuration

This document describes the GitHub Actions workflows and required configuration.

## Required Secrets

The following secrets must be configured in your GitHub repository settings (`Settings > Secrets and variables > Actions`):

### Docker Hub Publishing
- `DOCKERHUB_USERNAME` - Your Docker Hub username
- `DOCKERHUB_TOKEN` - Docker Hub access token (recommended) or password

**To create a Docker Hub access token:**
1. Go to https://hub.docker.com/settings/security
2. Click "New Access Token"
3. Give it a descriptive name (e.g., "GitHub Actions")
4. Copy the token and add it as `DOCKERHUB_TOKEN` secret

## Workflows

### CI Workflow (`.github/workflows/ci.yml`)
- Runs on every push and pull request
- Builds and tests the Rust code
- No secrets required

### Release Workflow (`.github/workflows/release.yml`)
- Triggered on tag pushes (`v*`)
- Builds multi-architecture binaries (Linux, macOS)
- Builds and pushes Docker images to:
  - GitHub Container Registry (`ghcr.io/deimosfr/custom-ddns`)
  - Docker Hub (`deimosfr/custom-ddns`)
- Creates GitHub releases with automatic changelog
- Requires: `DOCKERHUB_USERNAME`, `DOCKERHUB_TOKEN`

## Supported Architectures

**Binaries:**
- Linux: x86_64 (AMD64), aarch64 (ARM64)
- macOS: x86_64 (Intel), aarch64 (Apple Silicon)

**Docker Images:**
- Linux: amd64, arm64
- Available in both Debian (default) and Alpine variants

## Image Registries

Images are automatically published to both registries:

### GitHub Container Registry
- **URL**: `ghcr.io/deimosfr/custom-ddns`
- **Public**: Yes (after package is made public)
- **Authentication**: Uses `GITHUB_TOKEN` (automatic)

### Docker Hub
- **URL**: `deimosfr/custom-ddns`
- **Public**: Yes
- **Authentication**: Uses `DOCKERHUB_USERNAME` and `DOCKERHUB_TOKEN`

## Release Process

1. **Create a tag**: `git tag v1.0.0 && git push origin v1.0.0`
2. **Automatic builds**: GitHub Actions builds binaries and images
3. **Automatic release**: Creates GitHub release with changelog
4. **Multi-registry push**: Images pushed to both GHCR and Docker Hub

## Troubleshooting

### Failed Docker Hub Push
- Verify `DOCKERHUB_USERNAME` and `DOCKERHUB_TOKEN` secrets are set correctly
- Ensure the Docker Hub repository exists and you have push permissions
- Check the Docker Hub access token has appropriate permissions

### Failed GHCR Push
- The `GITHUB_TOKEN` is automatically provided by GitHub
- Ensure the repository has "write" permissions for packages
- Make sure the package visibility is set correctly

### Build Failures
- Check the workflow logs in the "Actions" tab
- Verify all dependencies are available
- Check for any breaking changes in dependencies 