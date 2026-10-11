# Spec Delta

## Purpose

Defines the architecture-specific container images and registry tags produced by continuous integration for users deploying the service on supported Linux hosts.

## ADDED Requirements

### Requirement: Publish separate container images for amd64 and arm64
The CI workflow MUST build and publish a Linux container image for each supported architecture.

#### Scenario: Publish images for a main branch update
- **WHEN** a commit is pushed to the default branch
- **THEN** CI publishes separate `linux/amd64` and `linux/arm64` images to the configured container registry

#### Scenario: Publish images for a version tag
- **WHEN** a matching version tag is pushed
- **THEN** CI publishes separate `linux/amd64` and `linux/arm64` images to the configured container registry

### Requirement: Keep amd64 tags and suffix arm64 tags
For each image publication, CI MUST retain existing amd64 tags and publish corresponding arm64 tags by appending `-arm64` to each tag.

#### Scenario: Publish default branch tags
- **WHEN** the default branch publishes its `latest` tag
- **THEN** the amd64 image is tagged `latest` and the arm64 image is tagged `latest-arm64`

#### Scenario: Publish version tags
- **WHEN** a version tag such as `v1.2.3` is published
- **THEN** the amd64 image is tagged `v1.2.3` and the arm64 image is tagged `v1.2.3-arm64`

#### Scenario: Publish commit SHA tags
- **WHEN** CI publishes a commit SHA tag
- **THEN** it retains the existing SHA tag for amd64 and publishes the corresponding SHA tag with `-arm64` appended for arm64

### Requirement: Build pull requests without publishing images
The workflow MUST validate both architecture builds for pull requests without publishing container images to the registry.

#### Scenario: Pull request targets the default branch
- **WHEN** a pull request targets the default branch
- **THEN** CI builds the amd64 and arm64 images and does not push either image to the registry
