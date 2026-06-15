# MPM: Molotov Package Manager Guide

MPM is the official package manager for the Molotov programming language. It allows you to download, manage, and use libraries hosted on GitHub.

## How it works

MPM is a GitHub-first package manager. It clones repositories directly from GitHub into a global library folder on your machine. The Molotov compiler (`mltv`) is configured to search this folder automatically when you use the `import` statement.

Libraries are stored under: `~/.molotov/libs/<user>/<repo>/`

Each package is namespaced by its GitHub username, so two different users can publish a library with the same name without any conflict. For example, `timmy/cool-library` and `pablo/cool-library` can both be installed and used at the same time.

## Installation

MPM is installed automatically when you run the main Molotov installation scripts:

### Linux / macOS
```bash
chmod +x install.sh && ./install.sh
```

### Windows (PowerShell)
```powershell
.\install.ps1
```

Once installed, the `mpm` command will be available in your terminal.

## Commands

### `install`: Install a package
Downloads a library from GitHub using the `user/repo` format:
```bash
mpm install jzadl/my-lib
```
If the package is already installed, MPM will let you know and suggest using `mpm update` instead.

### `update`: Update a package
Pulls the latest changes from GitHub for an already-installed package:
```bash
mpm update jzadl/my-lib
```

### `remove`: Remove a package
Deletes a library from your system. Requires the full `user/repo` format:
```bash
mpm remove jzadl/my-lib
```

### `list`: List installed packages
Shows all installed packages, grouped by user:
```bash
mpm list
```
Example output:
```
Installed packages in ~/.molotov/libs:
  jzadl/my-lib
  timmy/cool-library
  pablo/cool-library

  3 package(s) installed.
```

### `search`: Search installed packages
Filters your installed packages by name. Matches against both the username and the repo name:
```bash
mpm search cool-library
```
Example output:
```
Searching for 'cool-library':
  timmy/cool-library
  pablo/cool-library
```

### `help`: Show usage
```bash
mpm help
```

## Creating and Publishing Packages

Creating a Molotov package is simple:

1. Create a new public repository on GitHub.
2. Add your `.mltv` files to the repository.
3. If your package has a main entry point, name it the same as the repository (e.g., for repo `my-lib`, create `my-lib.mltv`).

Now anyone can install your library with:
```bash
mpm install your_username/your_repo
```

## Importing Packages

Packages are namespaced by `user/repo`, so imports use the double-colon `::` syntax.

### Importing the main module
If `timmy` has a repo called `hacking-library` containing `hacking-library.mltv`:
```python
import timmy::hacking-library
timmy::hacking-library.some_function()
```

Or with an alias to keep things short:
```python
import timmy::hacking-library as hacking-library
hacking-library.some_function()
```

### Importing specific submodules
If the repo contains other files, like `utils.mltv`, navigate into them with `::`:
```python
import timmy::hacking-library::utils
utils.helper()
```

### Handling name collisions
Since two users can have repos with the same name, you can install and use both simultaneously by aliasing them:
```python
import timmy::hacking-library as timmys_lib
import pablo::hacking-library as pablos_lib

timmys_lib.some_function()
pablos_lib.some_function()
```

### Common Pitfalls
- Always use the full `user/repo` format for `install`, `update`, and `remove`.
- If `import user::repo` fails, check that the repo contains a file named `repo.mltv` at its root.
- Submodule files must be referenced explicitly with `::`, they are not imported automatically.
