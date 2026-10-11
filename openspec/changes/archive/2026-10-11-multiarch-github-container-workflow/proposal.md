# Proposal

## Why

The current GitHub Actions workflow publishes a container image for only one architecture, which limits where the service can run. Publishing separate amd64 and arm64 images lets users select the image that matches their host while preserving the familiar existing tags for amd64.

## What Changes

- Build and publish Linux amd64 and arm64 container images from the existing GitHub Actions workflow.
- Keep current tags for amd64 and publish matching arm64 tags with an `-arm64` suffix, including `latest-arm64`, version tags such as `v1.2.3-arm64`, and suffixed SHA tags.
- Preserve the current pull request, main branch, and version tag workflow triggers and their existing publish behavior.

## Capabilities

### New Capabilities
- `container-image-publishing`: Defines the architecture-specific container images and tags published by CI.

### Modified Capabilities

## Impact

- Updates `.github/workflows/container.yml` to build and publish both target architectures.
- Documents the per-architecture GHCR tags available to downstream users.
- Uses the existing Dockerfile cross-compilation support and GitHub Actions permissions.
