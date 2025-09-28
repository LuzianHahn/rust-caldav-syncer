# Project Coding Rules (Non-Obvious Only)
- Use tokio::fs for all file operations to maintain async compatibility
- Config must be loaded from YAML using serde_yaml, no fallback to other formats
- Folder synchronization is recursive and includes all file types, not limited to images
- Incremental sync relies on SHA256 hashes computed with sha2 crate
- WebDAV operations use webdav crate with proper error handling for network issues