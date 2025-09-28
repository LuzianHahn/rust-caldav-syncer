# Project Architecture Rules (Non-Obvious Only)
- Synchronization targets WebDAV endpoint with incremental updates via local hash tracking
- Config specifies multiple root folders for recursive file synchronization
- Async architecture using tokio for concurrent file processing and network operations
- Hash-based change detection prevents unnecessary uploads of unchanged files
- Error handling must account for Android/Termux environment limitations