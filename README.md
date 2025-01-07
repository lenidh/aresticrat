
# Introduction
**Aresticrat** is a user-friendly wrapper for the Restic backup tool, streamlining backup management with the following key features:
- Configuration file: Simplifies backup setup by using a configuration file instead of requiring complex command-line options.
- Multi-repository/location: Supports backup of any number of locations to any number of repositories with a single command invocation.

## Quick start
1. Download the latest release binary for your system.
1. Move the binary to the desired installation directory and rename it to *aresticrat* (Linux/Unix) or *aresticrat.exe" (Windows).
1. Create a configuration file:
   1. Download the *aresticrat.example.toml* from the root of this repository.
   1. Move it to the installation directory.
   1. Rename it to *aresticrat.toml*.
   1. Define at least one location and one repository as described in the file.
1. Run `./aresticrat backup` (Linux/Unix) or `.\aresticrat.exe backup` (Windows).
