# MPM: Molotov Package Manager Guide

MPM is the official package manager for the Molotov programming language. It allows you to download, manage, and use libraries hosted on GitHub.

## How it works

MPM is a GitHub first package manager. It clones repositories directly from GitHub into a global library folder on your machine. The Molotov compiler (mltv) is configured to search this folder automatically when you use the `import` statement.

Libraries are stored in: `~/.molotov/libs/`

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

## Managing Packages

### Installing a package
To install a library, use the `install` command with the GitHub `user/repo` format:
```bash
mpm install jzadl/molotov
```
If the package is already installed, MPM will automatically perform a `git pull` to update it to the latest version.

### Listing installed packages
To see all the libraries you have downloaded:
```bash
mpm list
```

### Removing a package
To delete a library from your system:
```bash
mpm remove molotov
```
Note: You only need to provide the repository name (the part after the slash).

## Creating and Publishing Packages

Creating a Molotov package is simple. Follow these steps:

1. Create a new public repository on GitHub.
2. Add your `.mltv` files to the repository.
3. (Optional but recommended) If your package has a main entry point, name it the same as your repository (e.g., `my_lib/my_lib.mltv`).

Now anyone can install your library using `mpm install your_user/your_repo`.

## Importing Packages

This is the most important part to avoid errors. When you install a package like `jzadl/molotov`, MPM creates a folder named `molotov` in your libraries directory.

### Importing the main module
If the repository contains a file named `molotov.mltv`, you can import it directly:
```python
import molotov
molotov.hello()
```

### Importing specific files (Submodules)
If the repository contains other files, such as `hello.mltv`, you must use the double colon `::` syntax to navigate into the package folder:
```python
import molotov::hello
hello.hello()
```

### Common Pitfalls
If you try to use `import molotov` but there is no `molotov.mltv` file inside the `molotov` folder, the compiler will fail. Always check the file structure of the library you are using.
