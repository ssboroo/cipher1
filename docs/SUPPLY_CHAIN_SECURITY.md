# Rust Dependency Supply-Chain Controls

## BLAKE3 and the arrayref incident

CIPHER-1 pins BLAKE3 to version 1.8.7. This release removes the arrayref
dependency. The Rust Security Response Team reported that arrayref 0.3.10 had
been republished to depend on a malicious proc-macro1 build script and listed
the affected crate versions in its August 20, 2026 advisory:

https://blog.rust-lang.org/2026/08/20/supply-chain-attack-on-arrayref/

BLAKE3's upstream 1.8.7 release notes explicitly state that arrayref was
removed because its crates.io maintainer account had been compromised:

https://github.com/BLAKE3-team/BLAKE3/releases/tag/1.8.7

BLAKE3 1.8.7 uses the Rust 2024 edition and requires Rust 1.85. The workspace
MSRV is therefore 1.85; this is a deliberate supply-chain security trade-off,
not an accidental toolchain increase.

## CI controls

The Rust CI workflow:

- Uses exact versions for BLAKE3, XChaCha20-Poly1305, getrandom, and zeroize.
- Checks the runtime library MSRV with Rust 1.85.
- Builds Linux, Windows, and macOS runners.
- Cross-checks WebAssembly, Android ARM64, iOS ARM64, and Windows ARM64 targets.
- Scans the resolved all-target dependency tree for known-compromised versions
  of arrayref, internment, append-only-vec, proc-macro1, and related typosquats.

The dependency scan is defense in depth. It is not a substitute for reviewing
dependency updates, verifying crate checksums, using a committed lockfile for
applications, or running a dedicated advisory scanner.

## Operational policy

- Do not downgrade BLAKE3 below 1.8.7 to recover an older MSRV.
- Review transitive dependency changes before accepting automated updates.
- Reject unexpected new build scripts, network access, or executable build
  dependencies.
- Keep CI tokens read-only unless a workflow explicitly needs write access.
