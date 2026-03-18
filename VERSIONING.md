# Versioning policy

We follow [SemVer](https://semver.org/) across all packages: `ascend-tools` on PyPI, `ascend-tools` on npm, and the four crates on crates.io. All packages share a single version number, bumped in lockstep via `bin/bump-version`.

## What counts as breaking

"Breaking change" is inherently ambiguous for a project that spans a Rust SDK, Python SDK, JavaScript SDK, CLI, and MCP server. A change to CLI output format is breaking for scripts but not for SDK consumers. A new required field on a struct is breaking in Rust but invisible in Python/JS (where everything is a dict).

We err on the side of bumping the major version. If a change is breaking in *any* interface, we treat it as breaking for *all* of them — even if the other interfaces are technically unaffected. This keeps version numbers synchronized and avoids a world where "2.3.0 on PyPI is compatible with 1.8.0 on npm but not 2.1.0 on crates.io."

The tradeoff is that individual packages will occasionally get major bumps for changes that don't affect them directly. We think this is less confusing than version matrices.

## Guidelines

- **Major**: Any removal or behavioral change to a public API method, CLI command, MCP tool, or authentication flow. Any change to default output format.
- **Minor**: New methods, commands, tools, or optional parameters. New output fields (additive).
- **Patch**: Bug fixes, performance improvements, documentation, internal refactors with no user-visible change.

## Release process

`bin/release` tags the repo and pushes. GitHub Actions then publishes to all registries from a single `v{VERSION}` tag. Each release lands simultaneously on:

- **GitHub Releases** — changelog and prebuilt binaries
- **PyPI** — `ascend-tools` wheel (via maturin, trusted publishing)
- **npm** — `ascend-tools` with per-platform native addons
- **crates.io** — `ascend-tools-core`, `ascend-tools-cli`, `ascend-tools-mcp`, `ascend-tools-tui`

There is one tag, one GitHub Release, and one version number across all of them.
