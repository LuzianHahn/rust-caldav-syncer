# Project Documentation Rules (Non-Obvious Only)
- Configuration uses YAML format with specified folder paths for synchronization
- Folders in config are root directories to sync recursively to WebDAV
- Target platform is Termux on Android, affecting file paths and permissions
- Incremental sync tracks changes via local SHA256 hash storage