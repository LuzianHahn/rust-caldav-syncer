


## How to use this with Termux?
- get rustc via `pkg install rust`
- get compiler related ressources: `pkg install libllvm libllvm-static`
- install via `cargo install --git https://github.com/LuzianHahn/rust-caldav-syncer.git`


## TODOs
- use fast-hashing, just looking at metadata instead of the full file (faster, but less reliable)
  - [x] add tests
  - [ ] don't overwrite hash_store but extend it. (Why was clean `dummy_hash_store.yaml` prefilled with unrelated content from `dummy_target`folder and not just `dummy_target2`)
  - [ ] consider problem of outdated hashes? Is this really a problem?

- [ ] problems after syncing hash_store locally, even when remote one was deleted?
- [ ] `test`: only start test server once
- [ ] `feat`: sync from remote to local
- [ ] `perf`: do parallelized syncs
- [x] `fix`: hash_store updates after every synced file
- [ ] `fix`: `Drop` implementation around hash_store to always sync it, when the resource is freed
  - check for local hash_store!
- [ ] `fix`: check why local hash_store.yaml contains everything after removing it before remote sync.


## Untested Features
- 19e5d622d47e153773d20b27c7b571c23bc78238 - using a tempfile to clone a remote `hash_store.yaml`
  - manually tested by creating the default `hashes,yaml` with unrelated content and checking if its content is synced to a not existing remote `hash_store.yaml`
- 2ba128d19c534605b51c4de39ef041bfc23cb7dc - sync remote hash_store file even during interrupts.
  - manually tested by interrupting slow sync via keyboard interrupt and checking remote hash_store file, which file hashes it contains.
