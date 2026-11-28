# Evidence — Task 8: RoomEncryption on RoomSummary
Commit: 7b5525c (fixup of a459248)
Environment: local cargo, CC=clang-22 CXX=clang++-22. No Docker, no homeserver started.

## RoomSummary gains encryption: RoomEncryption {Encrypted, NotEncrypted, Unknown} mapped from SDK EncryptionState; unknown maps to Unknown; is_encrypted == (encryption == Encrypted); Default is Unknown
Status: PROVEN

```console
$ cargo test -p twicecat-core room_list::   (filtered to test lines)
test room_list::tests::each_sdk_encryption_state_maps_to_one_room_encryption ... ok
(22 other room_list tests ok)
test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 691 filtered out
$ cargo test -p twicecat-core --doc RoomEncryption
test crates/core/src/api/types.rs - api::types::RoomEncryption (line 315) ... ok
```
The mapping test asserts Encrypted->Encrypted, NotEncrypted->NotEncrypted, Unknown->Unknown, and that a fresh RoomSummary has encryption Unknown and is_encrypted false. The doctest asserts Default is Unknown. In `summarize`, `is_encrypted` is computed as `encryption == RoomEncryption::Encrypted`, so the two cannot diverge. `encryption_of` maps any other variant to Unknown.

## Additive only; scripts/check-core-api.sh passes
Status: PROVEN

```console
$ scripts/check-core-api.sh; echo "core-api exit=$?"
core-api exit=0
$ scripts/check-app-deps.sh; echo "app-deps exit=$?"
app-deps exit=0
$ cargo check -p twicecat-app --all-targets
    Finished `dev` profile ...
exit=0
```
Both scripts exit 0. The app crate and its tests compile unchanged. They still read `is_encrypted` at room_list_pane.rs:224, main_view.rs:236 and 283, and upload_dialog.rs:183. `git status` was clean apart from this file.

## Plan check: twicecat-cli against the local homeserver
Status: NOT PROVEN (not run: needs Docker; human checkpoint)
