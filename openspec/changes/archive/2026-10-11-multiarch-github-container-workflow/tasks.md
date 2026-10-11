# Tasks

## 1. Configure architecture matrix and image tags

- [x] 1.1 Update `.github/workflows/container.yml` to build `linux/amd64` and `linux/arm64` as separate matrix jobs; verify workflow structure parses and each job selects its intended platform.
- [x] 1.2 Preserve metadata-generated amd64 tags and append `-arm64` to every arm64 tag; verify `latest`, a version ref, and a SHA ref resolve to distinct expected tags for each architecture.

## 2. Preserve trigger and publishing behavior

- [x] 2.1 Retain pull request builds without registry login or pushes, and publish both architecture tag sets for existing main and version-tag events; verify event conditionals and permissions in the workflow.
- [x] 2.2 Validate the workflow YAML and confirm an amd64 and arm64 Buildx build complete successfully before merge.
