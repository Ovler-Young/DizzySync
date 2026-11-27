# DizzySync v2.0.0

DizzySync v2.0.0 is a major usability release focused on multi-account setup, the Web UI, local library visibility, and safer playback/download state reporting.

## Highlights

- **Multi-account support**: configure and validate multiple Dizzylab accounts from the Web UI.
- **Web setup and control**: first-run onboarding, protected API/Web UI access, sync controls, status cards, and log viewing.
- **Purchased albums view**: browse owned albums with table/card views, search, configurable columns, release dates, track counts, local state, and sync actions.
- **Local playback**: global audio player with queue, cover art, previous/next controls, and loop modes.
- **Album detail drawer**: metadata, local file diagnostics, compact sortable track list, per-track play buttons, and duration display when provided by the API.
- **Better local state detection**: downloaded/partial/missing states now distinguish actual local playable audio from directories or gift-only content.

## Release-blocking fixes included

- Albums with zero local audio tracks no longer show as playable.
- Gift content is tracked separately and is never counted as a playable audio file or audio format.
- Fully synced albums no longer show as partial only because a single audio file per track exists instead of one file per configured format.
- Missing configured audio formats are still reported when detailed album metadata is available.
- Album list release dates are enriched from detail/cache metadata or local README/NFO metadata when the list API omits them.
- Detail track rows avoid awkward wrapping and can be sorted by track number, name, duration, and local status.

## Assets

Download the ZIP that matches your platform:

- `dizzysync-x86_64-unknown-linux-gnu.zip`
- `dizzysync-aarch64-unknown-linux-musl.zip`
- `dizzysync-x86_64-pc-windows-msvc.zip`
- `dizzysync-aarch64-pc-windows-msvc.zip`
- `dizzysync-x86_64-apple-darwin.zip`
- `dizzysync-aarch64-apple-darwin.zip`

Each archive includes the `dizzysync` binary (or `dizzysync.exe` on Windows) and a sample `config.toml`.

## Validation

The release workflow runs Rust formatting, clippy, tests, Web UI lint/typecheck/build, Docker image build/publish, and multi-platform binary builds before publishing release assets.
