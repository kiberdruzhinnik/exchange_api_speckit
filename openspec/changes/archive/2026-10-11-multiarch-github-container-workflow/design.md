# Design

## Context

See proposal.md for motivation. The current `.github/workflows/container.yml` uses Buildx and GHCR, creates `latest`, version-ref, and SHA metadata tags, and skips login/push on pull requests. The Dockerfile already selects cross-compilers based on `TARGETARCH` and produces the service binary for the requested target.

## Goals / Non-Goals

**Goals:**
- Produce independent `linux/amd64` and `linux/arm64` images for the existing workflow events.
- Keep the current unsuffixed amd64 tags and derive arm64 tags by appending `-arm64`.
- Validate both platform builds on pull requests without publishing them.

**Non-Goals:**
- Publish a combined multi-platform manifest under an unsuffixed tag.
- Change Dockerfile behavior, GHCR naming, release triggers, or application runtime behavior.

## Decisions

### Build each architecture as a separate matrix job

Use a GitHub Actions matrix with one platform per job (`linux/amd64` and `linux/arm64`). Give the amd64 job an empty tag suffix and the arm64 job the `-arm64` suffix. This produces two independently addressable images and avoids requiring manifest assembly or QEMU execution. A single Buildx invocation for both platforms was considered, but it would normally publish a multi-platform manifest under common tags, contrary to the requested per-architecture tag scheme.

### Apply suffixes to all metadata tags

Continue deriving tags with the existing metadata action, then append the matrix suffix to each generated tag before passing tags to the build-push action. This preserves current amd64 `latest`, version, and SHA tags while giving arm64 matching tags such as `latest-arm64` and `<sha-tag>-arm64`. Applying the suffix uniformly avoids leaving arm64 SHA tags colliding with amd64 tags.

### Preserve event-specific push behavior

Run both matrix entries on the current pull request, default branch, and `v*.*.*` tag events. Keep registry login and image push disabled for pull requests; keep them enabled for main and version tag pushes. Continue using the existing repository-scoped GHCR image name and package permissions.

## Risks / Trade-offs

- [Risk] The two independent tags do not let Docker automatically choose an architecture from one shared tag. → Users select the matching tag explicitly; this is the requested tag contract.
- [Risk] Cross-compilation or a base image may not support one target. → Build both targets in pull request runs so incompatibilities are caught before publication.
- [Risk] Matrix jobs can publish partially if one architecture fails after the other succeeds. → Consumers should treat each architecture tag independently; a later successful workflow run updates both tags.

## Migration Plan

No data migration is needed. After merging, new default-branch and version-tag runs will publish both architecture-specific tag sets. Existing amd64 tags continue to be updated as before. Rollback consists of reverting the workflow change; already published arm64 tags may remain in GHCR until separately removed.
