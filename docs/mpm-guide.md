# Molotov Package Manager (mpm)

mpm is a package manager for Molotov projects. It installs packages from GitHub into `~/.molotov/libs/` using `.mpk` metadata files for versioning, authorship, and dependency tracking.

---

## Quick Start

### Install a package

```
mpm install user/repo
mpm install user/repo@1.0.0
mpm install user/repo/subpath
mpm install user/repo/subpath@1.0.0
```

### List installed packages

```
mpm list
```

### Search through installed packages

```
mpm search query
```

### Update a package

```
mpm update user/repo
mpm update user/repo/subpath
```

### Remove a package

```
mpm remove user/repo
mpm remove user/repo/subpath
```

---

## Package Format (.mpk)

Each package directory must contain a `package.mpk` file in dot notation. Lines use the form `key value`. Comments start with `#`. Quoted strings use double quotes `""`.

Example:

```
package.name        greeter
package.version     1.2.3
package.author      jzadl
package.description "Welcome and farewell messages"

deps.math-utils     molotov/math-utils@1.0.0
deps.str-utils      molotov/str-utils@0.5.0
```

### Supported fields

| Field | Description |
| --- | --- |
| `package.name` | Package display name |
| `package.version` | Semantic version string |
| `package.author` | Author name or handle |
| `package.description` | Short description in double quotes |
| `deps.*` | Dependency declaration (see below) |

### Whitespace rules

Actual `.mpk` files may use multiple spaces or tabs between the key and the value, not just a single space.

---

## Package Source Format

Packages can ship either `.rs` (native Rust) or `.mltv` (Molotov) source files. The transpiler resolves `from pkg import func` by looking for `pkg.rs` or `pkg.mltv` in `~/.molotov/libs/pkg/`.

For libraries that expose functions to Molotov programs, `.rs` is the recommended format. The `.rs` file contains plain Rust `pub fn` with no special wrappers. The transpiler includes it via `pub mod pkg { include!("path/to/pkg.rs"); }` and generates `use pkg::{func}` for the user program.

For `.rs` libraries, only the Rust standard library is available unless you add external crates to Molotov's generated `Cargo.toml`. The `rand` and `serde_json` crates are included by default. Functions must return types that map to Molotov: `i64`, `f64`, `bool`, `String`, `Vec<T>`, `HashMap<String, V>`.

---

## Subpath Packages

A single GitHub repository can host multiple packages in subdirectories. The format is:

```
mpm install user/repo/subpackage
mpm install user/repo/a/b/c
```

When you install `user/repo/subpath`, mpm clones `github.com/user/repo.git` to `~/.molotov/libs/user/repo/` and reads metadata from `subpath/package.mpk`.

---

## Dependency Resolution

Dependencies are declared with `deps.*` lines in `package.mpk`:

```
deps.name     user/repo[@version]
```

Dependency specs follow the same format as install targets. When a dependency is declared, it is resolved relative to the parent package's monorepo.

Example: If `jzadl/mpk-libtests/greeter` declares:

```
deps.math-utils     molotov/math-utils@1.0.0
```

Then mpm installs `jzadl/mpk-libtests/molotov/math-utils@1.0.0` (a subpath within the same repository clone) instead of trying to fetch `github.com/molotov/math-utils.git` separately.

### How it works

1. The parent package's repo is cloned once.
2. Dependencies with `user/repo` format are treated as subpaths inside the same clone.
3. The repo is not re-cloned for dependency installations.
4. If a version is specified, git attempts to check out the corresponding branch or tag.

---

## Directory Layout

After installation, the filesystem looks like:

```
~/.molotov/libs/
  user/
    repo/
      package.mpk
      repo.rs
      subpackage/
        package.mpk
        subpackage.rs
      another-sub/
        package.mpk
        another-sub.mltv
```

Each package directory contains a `package.mpk` for metadata and either a `.rs` or `.mltv` source file with the same name as the package. The transpiler finds the source file via `~/.molotov/libs/<name>/<name>.rs` or `~/.molotov/libs/<name>/<name>.mltv` when you write `from <name> import ...`.

---

## Commands

### install

Clones a GitHub repository and reads package metadata.

```
mpm install user/repo[/subpath][@version]
```

Steps:
1. Parse the package spec into user, repo, subpath, and version.
2. Clone `https://github.com/user/repo.git` to `~/.molotov/libs/user/repo/`.
3. If no version is specified, check out the latest git tag.
4. Display metadata from `package.mpk` (version, author, description).
5. Scan for dependencies declared in `package.mpk` and install them.
6. If the repo already exists, skip cloning and just check out the requested version.

### list

Lists all installed packages with their version and author metadata.

```
mpm list
```

Scans `~/.molotov/libs/` for all directories containing `package.mpk`, including subdirectory packages. Displays each package with its version and author.

### search

Filters installed packages by a query string.

```
mpm search query
```

Matches against user name, repo name, and subpath name. Shows matching packages with version and author metadata.

### update

Updates an installed package via git pull.

```
mpm update user/repo[/subpath][@version]
```

Steps:
1. Run `git pull` in the repository directory.
2. If a version was specified, check out that branch or tag.
3. If no version was specified, check out the latest tag.
4. Display updated metadata.

### remove

Removes the entire repository directory.

```
mpm remove user/repo
```

Since subpath packages share a repository, removing `user/repo` removes all subpackages within it.

---

## Building mpm

mpm is written in Molotov and compiled through the Molotov transpiler (mltv).

```
cd /home/jzadl/molotov
cargo run -- deploy mpm.mltv -o mpm
./mpm install user/repo
```

### install.sh

The project includes an `install.sh` script that builds both mltv and mpm and installs them to `~/.local/bin/`.

```
./install.sh
./install.sh --no-destruct
```

The `--no-destruct` flag skips the destruction phase.
---

## libtest

The project includes `libtest.mltv` which tests core library operations:

- File existence checks with `exists()`
- `package.mpk` parsing: version, author, description
- Dependency declaration detection

Run it with:

```
./libtest
```

Or rebuild it with:

```
cargo run -- deploy libtest.mltv -o libtest
```

---

## Test Repo

The test repository is at `https://github.com/jzadl/mpk-libtests` and contains four packages in subdirectories:

| Package | Path | Version | Dependencies |
| --- | --- | --- | --- |
| greeter | `greeter/` | 1.2.3 | molotov/math-utils@1.0.0, molotov/str-utils@0.5.0 |
| hello-lib | `hello-lib/` | 2.1.0 | molotov/str-utils@0.5.0 |
| math-utils | `math-utils/` | 1.0.0 | none |
| str-utils | `str-utils/` | 0.5.0 | none |

---

## File Reference

| File | Purpose |
| --- | --- |
| `mpm.mltv` | Main package manager source |
| `libtest.mltv` | Library operation tests |
| `mpm-guide.md` | This guide |
| `mpk_context.md` | Original mpk format draft |
| `mpm.rs` | Transpiled Rust output (generated) |
| `mpm` | Compiled binary (generated) |
| `libtest` | Compiled test binary (generated) |
