# Contributing

Thanks for contributing to Ruzzle OS. Please follow the guidelines below.

## Branching (Git Flow)
- `main`: production branch, no direct pushes
- `develop`: integration branch for the next release
- Branch naming: `type/description`
  - `feature/*`, `fix/*`, `release/*`, `hotfix/*`, `refactor/*`, `chore/*`

## Commit Message Format
Format: `type: :gitmoji: description`
- Example: `feat: ✨ add usb keyboard input`

## Code Rules
- Architecture: Presentation → Domain ← Data (Clean Architecture)
- Naming/formatting: follow the project standards
- Warnings/Clippy errors are not allowed

## Tests
- Run `cargo test` or the project test suite when applicable
- If coverage gates exist, they must pass

## Pull Requests
- Keep changes small and focused
- Include a summary and test results
- Link relevant issues
